use std::future::Future;
use std::pin::Pin;

use futures::FutureExt;
use futures::{channel::mpsc, AsyncRead, AsyncWrite, SinkExt, StreamExt};
use thiserror::Error;
use tokio::time;
use tower::{Service, ServiceExt};

use cuprate_address_book::{AddressBookError, AddressBookRequest, AddressBookResponse};
use cuprate_common::{HardForks, Network};
use cuprate_protocol::temp_database::{DataBaseRequest, DataBaseResponse, DatabaseError};
use cuprate_protocol::{
    Direction, InternalMessageRequest, InternalMessageResponse, P2P_MAX_PEERS_IN_HANDSHAKE,
};
use monero_wire::{
    levin::{BucketError, MessageSink, MessageStream},
    messages::{
        admin::{HandshakeRequest, HandshakeResponse},
        common::PeerSupportFlags,
        BasicNodeData, CoreSyncData, MessageResponse, PeerID, PeerListEntryBase,
    },
    Message, NetworkAddress,
};
use tracing::Instrument;

use crate::{
    connection::{ClientRequest, Connection, ConnectionInfo, PeerSyncChange},
    PeerError,
};

#[derive(Debug, Error)]
pub enum HandShakeError {
    #[error("The peer has a weird pruning scheme")]
    PeerClaimedWeirdPruning,
    #[error("The peer has an unexpected top version")]
    PeerHasUnexpectedTopVersion,
    #[error("The peer does not have the minimum support flags")]
    PeerDoesNotHaveTheMinimumSupportFlags,
    #[error("Address book err: {0}")]
    AddressBookError(#[from] AddressBookError),
    #[error("The peer sent too many peers, considered spamming")]
    PeerSentTooManyPeers,
    #[error("The peer sent a wrong response to our handshake")]
    PeerSentWrongResponse,
    #[error("The syncer returned an error")]
    DataBaseError(#[from] DatabaseError),
    #[error("Bucket error while communicating with peer: {0}")]
    BucketError(#[from] BucketError),
}

pub enum AddressBookUpdate {
    NewPeers(Vec<PeerListEntryBase>),
    WhiteList(NetworkAddress),
    RemovePeer(NetworkAddress),
    BanPeer(NetworkAddress),
    AnchorPeer(NetworkAddress),
}

pub struct NetworkConfig {
    /// Port
    my_port: u32,
    /// The Network
    network: Network,
    /// Peer ID
    peer_id: PeerID,
    /// RPC Port
    rpc_port: u16,
    /// RPC Credits Per Hash
    rpc_credits_per_hash: u32,
    our_support_flags: PeerSupportFlags,
    minimum_peer_support_flags: PeerSupportFlags,
    max_in_peers: u32,
    target_out_peers: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            my_port: 18080,
            network: Network::MainNet,
            peer_id: PeerID(21),
            rpc_port: 0,
            rpc_credits_per_hash: 0,
            our_support_flags: PeerSupportFlags::get_support_flag_fluffy_blocks(),
            minimum_peer_support_flags: PeerSupportFlags::from(0_u32),
            max_in_peers: 13,
            target_out_peers: 21,
        }
    }
}

impl NetworkConfig {
    pub fn basic_node_data(&self) -> BasicNodeData {
        BasicNodeData {
            my_port: self.my_port,
            network_id: self.network.network_id(),
            peer_id: self.peer_id,
            support_flags: self.our_support_flags,
            rpc_port: self.rpc_port,
            rpc_credits_per_hash: self.rpc_credits_per_hash,
        }
    }
}

pub struct Handshake<W, R> {
    sink: MessageSink<W, Message>,
    stream: MessageStream<R, Message>,
    direction: Direction,
    addr: NetworkAddress,
}

pub struct Handshaker<Bc, Svc, AdrBook> {
    config: NetworkConfig,
    parent_span: tracing::Span,
    address_book: AdrBook,
    blockchain: Bc,
    peer_sync_states: mpsc::Sender<PeerSyncChange>,
    peer_request_service: Svc,
}

impl<Bc, Svc, AdrBook, W, R> tower::Service<Handshake<W, R>> for Handshaker<Bc, Svc, AdrBook>
where
    Bc: Service<DataBaseRequest, Response = DataBaseResponse, Error = DatabaseError>
        + Clone
        + Send
        + 'static,
    Bc::Future: Send,

    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = PeerError>
        + Clone
        + Send
        + 'static,

    AdrBook: Service<AddressBookRequest, Response = AddressBookResponse, Error = AddressBookError>
        + Clone
        + Send
        + 'static,
    AdrBook::Future: Send,

    W: AsyncWrite + std::marker::Unpin + Send + 'static,
    R: AsyncRead + std::marker::Unpin + Send + 'static,
{
    type Error = HandShakeError;
    type Response = ();
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Handshake<W, R>) -> Self::Future {
        let Handshake {
            sink: mut peer_sink,
            stream: mut peer_stream,
            direction,
            addr,
        } = req;

        let span = tracing::debug_span!("Handshaker");

        let mut blockchain = self.blockchain.clone();
        let mut address_book = self.address_book.clone();
        //let mut syncer_tx = self.peer_sync_states.clone();
        let basic_node_data = self.config.basic_node_data();
        let minimum_support_flags = self.config.minimum_peer_support_flags.clone();

        match direction {
            Direction::Outbound => {
                tracing::debug!(
                    parent: &span,
                    "Initiating outbound handshake with peer {addr:?}"
                );

                async move {
                    let our_core_sync = get_our_core_sync(&mut blockchain).await?;

                    send_handshake_req(basic_node_data, our_core_sync, &mut peer_sink).await?;

                    let HandshakeResponse {
                        node_data: peer_node_data,
                        payload_data: peer_core_sync,
                        local_peerlist_new,
                    } = get_handshake_res(&mut peer_stream).await?;

                    if !peer_node_data
                        .support_flags
                        .contains(&minimum_support_flags)
                    {
                        tracing::debug!(
                            "Handshake failed: peer does not have minimum support flags"
                        );
                        return Err(HandShakeError::PeerDoesNotHaveTheMinimumSupportFlags);
                    }

                    if local_peerlist_new.len() > P2P_MAX_PEERS_IN_HANDSHAKE {
                        tracing::debug!("Handshake failed: peer sent too many peers in response");
                        return Err(HandShakeError::PeerSentTooManyPeers);
                    }

                    // Tell the address book about the new peers
                    address_book
                        .ready()
                        .await?
                        .call(AddressBookRequest::HandleNewPeerList(
                            local_peerlist_new,
                            addr.get_zone(),
                        ))
                        .await?;

                    Ok(())
                }
                .instrument(span)
                .boxed()
            }
            Direction::Inbound => todo!(),
        }
    }
}

async fn get_our_core_sync<Bc>(blockchain: &mut Bc) -> Result<CoreSyncData, DatabaseError>
where
    Bc: Service<DataBaseRequest, Response = DataBaseResponse, Error = DatabaseError>,
{
    let blockchain = blockchain.ready().await?;
    let DataBaseResponse::CoreSyncData(core_sync) = blockchain.call(DataBaseRequest::CoreSyncData).await? else {
        unreachable!("Database will always return the requested item")
    };
    Ok(core_sync)
}

async fn send_handshake_req<W: AsyncWrite + std::marker::Unpin>(
    node_data: BasicNodeData,
    payload_data: CoreSyncData,
    peer_sink: &mut MessageSink<W, Message>,
) -> Result<(), HandShakeError> {
    let handshake_req = HandshakeRequest {
        node_data,
        payload_data,
    };

    tracing::trace!("Sending handshake request: {handshake_req:?}");

    let message: Message = Message::Request(handshake_req.into());
    peer_sink.send(message).await?;
    Ok(())
}

async fn get_handshake_res<R: AsyncRead + std::marker::Unpin>(
    peer_stream: &mut MessageStream<R, Message>,
) -> Result<HandshakeResponse, HandShakeError> {
    // put a timeout on this
    let Message::Response(MessageResponse::Handshake(handshake_res)) =  peer_stream.next().await.expect("MessageSink will not return None")? else {
        return Err(HandShakeError::PeerSentWrongResponse);
    };

    tracing::trace!("Received handshake response: {handshake_res:?}");

    Ok(handshake_res)
}

// impl<Bc, Svc> Handshaker<Bc, Svc>
// where
//     Bc: Service<DataBaseRequest, Response = DataBaseResponse, Error = DatabaseError>,
//     Svc:
//         Service<InternalMessageRequest, Response = InternalMessageResponse, Error = PeerError> + Clone + Send + 'static,
// {

//     async fn get_our_core_sync(&mut self) -> Result<CoreSyncData, DatabaseError> {
//         let blockchain = self.blockchain.ready().await?;
//         let DataBaseResponse::CoreSyncData(core_sync) = blockchain.call(DataBaseRequest::CoreSyncData).await? else {
//             unreachable!("Database will always return the requested item")
//         };
//         Ok(core_sync)
//     }
//     async fn send_handshake_req<W: AsyncWrite + std::marker::Unpin>(
//         &mut self,
//         peer_sink: &mut MessageSink<W, Message>,
//     ) -> Result<(), HandShakeError> {
//         let handshake_req = HandshakeRequest {
//             node_data: self.config.basic_node_data(),
//             payload_data: self.get_our_core_sync().await?,
//         };
//         let message: Message = Message::Request(handshake_req.into());
//         peer_sink.send(message).await?;
//         Ok(())
//     }

//     async fn get_handshake_res<R: AsyncRead + std::marker::Unpin>(
//         &mut self,
//         peer_stream: &mut MessageStream<Message, R>,
//     ) -> Result<HandshakeResponse, HandShakeError> {
//         // put a timeout on this
//         let Message::Response(MessageResponse::Handshake(handshake_res)) =  peer_stream.next().await.expect("MessageSink will not return None")? else {
//             return Err(HandShakeError::PeerSentWrongResponse);
//         };
//         Ok(handshake_res)
//     }

//     pub async fn complete_handshake<R, W>(
//         &mut self,
//         peer_reader: R,
//         peer_writer: W,
//         direction: Direction,
//         addr: NetworkAddress,
//     ) -> Result<(mpsc::Sender<ClientRequest>, Connection<Svc, W, R>), HandShakeError>
//     where
//         R: AsyncRead + std::marker::Unpin,
//         W: AsyncWrite + std::marker::Unpin,
//     {
//         let mut peer_sink = MessageSink::new(peer_writer);
//         let mut peer_stream = MessageStream::new(peer_reader);
//         let (c, conn) = match direction {
//             Direction::Outbound => {
//                 self.send_handshake_req(&mut peer_sink).await?;
//                 let handshake_res = self.get_handshake_res(&mut peer_stream).await?;
//                 if !handshake_res
//                     .node_data
//                     .support_flags
//                     .contains(&self.config.minimum_peer_support_flags)
//                 {
//                     return Err(HandShakeError::PeerDoesNotHaveTheMinimumSupportFlags);
//                 }
//                 if handshake_res.local_peerlist_new.len() > P2P_MAX_PEERS_IN_HANDSHAKE {
//                     return Err(HandShakeError::PeerSentTooManyPeers);
//                 }

//                 self.address_book_tx
//                     .send(AddressBookUpdate::NewPeers(handshake_res.local_peerlist_new))
//                     .await;

//                 let connection_info = ConnectionInfo { addr };

//                 let (client_tx, client_rx) = mpsc::channel(2);

//                 let con = Connection::new(
//                     connection_info,
//                     peer_sink,
//                     peer_stream,
//                     client_rx,
//                     self.peer_sync_states.clone(),
//                     self.peer_request_service.clone(),
//                 );
//                 (client_tx, con)
//             },
//             Direction::Inbound => {
//                 todo!()
//             },
//         };

//         Ok((c, conn))
//     }
// }
