use std::collections::HashSet;

use futures::{AsyncRead, AsyncWrite, StreamExt, SinkExt};
use futures::stream::Fuse;
use futures::channel::{oneshot, mpsc};
use monero::Hash;

use monero::cryptonote::hash::Hashable;
use monero::database::CRYPTONOTE_MAX_BLOCK_NUMBER;
use monero_wire::messages::{BasicNodeData, PeerID, GetObjectsResponse, GetObjectsRequest, ProtocolMessage};
use monero_wire::messages::admin::HandshakeResponse;
use monero_wire::messages::common::PeerSupportFlags;
use monero_wire::{levin, Message, NetworkAddress};
use levin::{MessageSink, MessageStream};
use thiserror::__private::DisplayAsDisplay;
use tower::{Service, ServiceExt};

use crate::Network;
use crate::protocol::{InternalMessageRequest, InternalMessageResponse};
use super::{
    PeerError, PeerResponseError, BLOCKS_IDS_SYNCHRONIZING_MAX_COUNT, P2P_MAX_PEERS_IN_HANDSHAKE, RequestServiceError,
};

pub struct ClientRequest {
    req: InternalMessageRequest,
    tx: oneshot::Sender<Result<InternalMessageResponse, PeerError>>,
}

pub enum State {
    WaitingForRequest,
    WaitingForResponse {
        request: InternalMessageRequest,
        tx: oneshot::Sender<Result<InternalMessageResponse, PeerError>>,
    },
}

impl State {
    pub fn expected_response_id(&self) -> Option<u32> {
        match self {
            Self::WaitingForRequest => None,
            Self::WaitingForResponse { request, tx: _ } => request.expected_id(),
        }
    }
}

pub enum Direction {
    Inbound,
    Outbound,
}

pub struct PeerInfo {
    id: PeerID,
    port: u32,
    current_height: u64,
    cumulative_difficulty: u128,
    support_flags: PeerSupportFlags,
    pruning_seed: u32,
    rpc_port: u16,
    rpc_credits_per_hash: u32,
    direction: Direction,
}

pub struct Connection<Svc, Aw, Ar> {
    peer_info: PeerInfo,
    network: Network,
    state: State,
    sink: MessageSink<Aw, Message>,
    stream: Fuse<MessageStream<Message, Ar>>,
    client_rx: mpsc::Receiver<ClientRequest>,
    svc: Svc,
}

impl<Svc, Aw, Ar> Connection<Svc, Aw, Ar>
where
    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = RequestServiceError>,
    Aw: AsyncWrite + std::marker::Unpin,
    Ar: AsyncRead + std::marker::Unpin,
{
    async fn handle_response(&mut self, res: InternalMessageResponse) -> Result<(), PeerResponseError> {
        let state = std::mem::replace(&mut self.state, State::WaitingForRequest);
        if let State::WaitingForResponse { request, tx } = state {
            match (request, &res) {
                (InternalMessageRequest::Handshake(_), InternalMessageResponse::Handshake(_)) => {
                    // we are already connected to the peer
                    return Err(PeerResponseError::PeerSentHandshakeAgain);
                },
                (InternalMessageRequest::SupportFlags(_), InternalMessageResponse::SupportFlags(_)) => {
                    // we are already connected to the peer - this happens during handshakes
                    return Err(PeerResponseError::PeerSentHandshakeAgain);
                },
                (InternalMessageRequest::TimedSync(_), InternalMessageResponse::TimedSync(res)) => {
                    if res.local_peerlist_new.len() > P2P_MAX_PEERS_IN_HANDSHAKE {
                        return Err(PeerResponseError::PeerSentTooManyPeers);
                    }
                    if self.peer_info.cumulative_difficulty > res.payload_data.cumulative_difficulty() {
                        return Err(PeerResponseError::PeersCumulativeDifficultyHasDropped);
                    }
                    if self.peer_info.current_height > res.payload_data.current_height {
                        return Err(PeerResponseError::PeersHeightHasDropped);
                    }
                    if self.peer_info.pruning_seed != res.payload_data.pruning_seed {
                        return Err(PeerResponseError::PeersPruningSeedHasChanged);
                    }

                    self.peer_info.current_height = res.payload_data.current_height;
                    self.peer_info.cumulative_difficulty = res.payload_data.cumulative_difficulty();
                },
                (InternalMessageRequest::GetObjectsRequest(req), InternalMessageResponse::GetObjectsResponse(res)) => {
                    if req.blocks.len() != res.blocks.len() {
                        return Err(PeerResponseError::PeerSentIncorrectAmountOfBlocks);
                    }
                    if res.current_blockchain_height < self.peer_info.current_height {
                        return Err(PeerResponseError::PeersHeightHasDropped);
                    }
                    self.peer_info.current_height = res.current_blockchain_height;

                    let mut block_ids: HashSet<&monero::Hash> = HashSet::from_iter(req.blocks.iter());

                    // TODO: I couldn't see any code in monerod that actually checks for missed_ids so i guess we wont ether, is this a good idea?

                    // for ids in res.missed_ids.iter() {
                    //     let _ = !block_ids.remove(ids);
                    // }

                    for block in res.blocks.iter() {
                        if !req.pruned {
                            // we don't want pruned blocks
                            if block.pruned || block.block_weight != 0 || !block.txs_pruned.is_empty() {
                                return Err(PeerResponseError::PeerSentItemWithAPruningStateWeDoNotWant);
                            }
                            if block.txs.len() != block.block.tx_hashes.len() {
                                return Err(PeerResponseError::PeerSentBlockWithIncorrectTransactions);
                            }
                            let mut tx_ids: HashSet<&monero::Hash> = HashSet::from_iter(block.block.tx_hashes.iter());
                            for tx in block.txs.iter() {
                                if !tx_ids.remove(&tx.hash()) {
                                    return Err(PeerResponseError::PeerSentBlockWithIncorrectTransactions);
                                }
                            }
                            // no need to check the hash set is empty we just check the length of both tx hashes and txs
                        } else {
                            if block.block_weight == 0 && block.pruned {
                                return Err(PeerResponseError::PeerSentItemWithoutEnoughInformation);
                            }
                            if block.txs_pruned.len() != block.block.tx_hashes.len() {
                                return Err(PeerResponseError::PeerSentBlockWithIncorrectTransactions);
                            }
                            let mut tx_ids: HashSet<&monero::Hash> = HashSet::from_iter(block.block.tx_hashes.iter());
                            for tx in block.txs_pruned.iter() {
                                if !tx_ids.remove(
                                    &tx.hash()
                                        .map_err(|_| PeerResponseError::PeerSentBlockWithIncorrectTransactions)?,
                                ) {
                                    return Err(PeerResponseError::PeerSentBlockWithIncorrectTransactions);
                                }
                            }
                            // no need to check the hash set is empty we just check the length of both tx hashes and txs
                        }
                        if !block_ids.remove(&block.block.id()) {
                            return Err(PeerResponseError::PeerSentBlockWeDidNotRequest);
                        }
                    }
                },
                (InternalMessageRequest::ChainRequest(req), InternalMessageResponse::ChainResponse(res)) => {
                    if res.m_block_ids.is_empty() {
                        return Err(PeerResponseError::PeerSentNoBlockIds);
                    }
                    if res.total_height < res.m_block_ids.len() as u64
                        || res.start_height > res.total_height - res.m_block_ids.len() as u64
                    {
                        return Err(PeerResponseError::PeerSentBadStartOrNBlocksOrheight);
                    }
                    if req.prune && res.m_block_ids.len() != res.m_block_weights.len() {
                        return Err(PeerResponseError::PeerSentInvalidBlockWeights);
                    }
                    if res.total_height >= CRYPTONOTE_MAX_BLOCK_NUMBER
                        || res.m_block_ids.len() > BLOCKS_IDS_SYNCHRONIZING_MAX_COUNT
                    {
                        return Err(PeerResponseError::PeerSentTooMuchInformation);
                    }
                    if res.total_height < self.peer_info.current_height {
                        return Err(PeerResponseError::PeersHeightHasDropped);
                    }
                    self.peer_info.current_height = res.total_height;
                },
                (
                    InternalMessageRequest::FluffyMissingTransactionsRequest(req),
                    InternalMessageResponse::NewFluffyBlock(blk),
                ) => {
                    // fluffy blocks will only be sent for new blocks at the top of the chain hence they won't be pruned hence we can just use blk.b.txs.
                    if req.missing_tx_indices.len() != blk.b.txs.len() {
                        return Err(PeerResponseError::PeerSentBlockWithIncorrectTransactions);
                    }
                    if blk.b.pruned {
                        return Err(PeerResponseError::PeerSentItemWithAPruningStateWeDoNotWant);
                    }
                    if req.block_hash != blk.b.block.id() {
                        return Err(PeerResponseError::PeerSentBlockWeDidNotRequest);
                    }
                    let mut new_tx_hashes: HashSet<monero::Hash> =
                        HashSet::from_iter(blk.b.txs.iter().map(|tx| tx.hash()));

                    let tx_ids = &blk.b.block.tx_hashes;
                    for idx in req.missing_tx_indices {
                        if let Some(tx) = tx_ids.get(idx as usize) {
                            if !new_tx_hashes.remove(tx) {
                                return Err(PeerResponseError::PeerSentBlockWithIncorrectTransactions);
                            }
                        } else {
                            // im pretty sure this if this happens it's a problem on our part
                            return Err(PeerResponseError::PeerSentBlockWithIncorrectTransactions);
                        }
                    }
                },
                (InternalMessageRequest::GetTxPoolCompliment(_), InternalMessageResponse::NewTransactions(_)) => {
                    // we could check we received no transactions that we said we knew about but thats going to happen later anyway when they get added to our
                    // mempool
                },
                _ => return Err(PeerResponseError::PeerSentWrongResponse),
            }
            // response passed our tests we can send it to the requestor
            let _ = tx.send(Ok(res));
            Ok(())
        } else {
            unreachable!("This will only be called when in state WaitingForResponse");
        }
    }

    async fn send_message_to_peer(&mut self, mes: impl Into<Message>) -> Result<(), PeerError> {
        Ok(self.sink.send(mes.into()).await?)
    }

    async fn handle_peer_request(&mut self, req: InternalMessageRequest) -> Result<(), PeerError> {
        // we should check contents of peer requests for obvious errors like we do with responses
        let ready_svc = self.svc.ready().await?;
        let res = ready_svc.call(req).await?;
        self.send_message_to_peer(res).await
    }

    async fn handle_client_request(&mut self, req: ClientRequest) -> Result<(), PeerError> {
        // check we need a response
        if let Some(_) = req.req.expected_id() {
            self.state = State::WaitingForResponse {
                request: req.req.clone(),
                tx: req.tx,
            };
        }
        self.send_message_to_peer(req.req).await
    }

    async fn state_waiting_for_request(&mut self) -> Result<(), PeerError> {
        futures::select! {
            peer_message = self.stream.next() => {
                match peer_message.expect("MessageStream will never return None") {
                    Ok(message) => {
                        self.handle_peer_request(message.try_into().map_err(|_| PeerError::PeerSentUnexpectedResponse)?).await
                    },
                    Err(e) => Err(e.into()),
                }
            },
            client_req = self.client_rx.next() => {
                self.handle_client_request(client_req.ok_or(PeerError::ClientChannelClosed)?).await
            },
        }
    }

    async fn state_waiting_for_response(&mut self) -> Result<(), PeerError> {
        // put a timeout on this
        let peer_message = self
            .stream
            .next()
            .await
            .expect("MessageStream will never return None")?;
        if !peer_message.is_request() && self.state.expected_response_id() == Some(peer_message.id()) {
            if let Ok(res) = peer_message.try_into() {
                Ok(self.handle_response(res).await?)
            } else {
                // im almost certain this is impossible to hit, but im not certain enough to use unreachable!()
                Err(PeerResponseError::PeerSentWrongResponse)?
            }
        } else {
            if let Ok(req) = peer_message.try_into() {
                self.handle_peer_request(req).await
            } else {
                // this can be hit if the peer sends a protocol response with the wrong id
                Err(PeerResponseError::PeerSentWrongResponse)?
            }
        }
    }

    pub async fn run(mut self) {
        loop {
            let _res = match self.state {
                State::WaitingForRequest => self.state_waiting_for_request().await,
                State::WaitingForResponse { request: _, tx: _ } => self.state_waiting_for_response().await,
            };
        }
    }
}
