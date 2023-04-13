use futures::{channel::mpsc, AsyncWrite, AsyncRead, SinkExt, StreamExt};
use monero_wire::{
    messages::{
        common::PeerSupportFlags,
        PeerListEntryBase,
        admin::{HandshakeRequest, HandshakeResponse},
        PeerID, BasicNodeData, CoreSyncData, MessageResponse,
    },
    NetworkAddress,
    levin::{MessageSink, MessageStream, BucketError},
    Message,
};
use thiserror::Error;
use tower::{Service, ServiceExt};

use cuprate_common::{Network, HardForks};

use cuprate_protocol::{InternalMessageRequest, InternalMessageResponse, P2P_MAX_PEERS_IN_HANDSHAKE, Direction};
use cuprate_protocol::temp_database::{DataBaseRequest, DataBaseResponse, DatabaseError};

use crate::{
    connection::{PeerSyncChange, Connection, ConnectionInfo, ClientRequest},
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
    #[error("The address book channel has closed")]
    AddressBookChannelClosed,
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

pub struct Handshaker<Bc, Svc> {
    config: NetworkConfig,
    address_book_tx: mpsc::Sender<AddressBookUpdate>,
    blockchain: Bc,
    peer_sync_states: mpsc::Sender<PeerSyncChange>,
    peer_request_service: Svc,
}

impl<Bc, Svc> Handshaker<Bc, Svc>
where
    Bc: Service<DataBaseRequest, Response = DataBaseResponse, Error = DatabaseError>,
    Svc:
        Service<InternalMessageRequest, Response = InternalMessageResponse, Error = PeerError> + Clone + Send + 'static,
{
    pub fn new(
        config: NetworkConfig,
        address_book_tx: mpsc::Sender<AddressBookUpdate>,
        blockchain: Bc,
        peer_sync_states: mpsc::Sender<PeerSyncChange>,
        peer_request_service: Svc,
    ) -> Self {
        Handshaker {
            config,
            address_book_tx,
            blockchain,
            peer_sync_states,
            peer_request_service,
        }
    }

    async fn get_our_core_sync(&mut self) -> Result<CoreSyncData, DatabaseError> {
        let blockchain = self.blockchain.ready().await?;
        let DataBaseResponse::CoreSyncData(core_sync) = blockchain.call(DataBaseRequest::CoreSyncData).await? else {
            unreachable!("Database will always return the requested item")
        };
        Ok(core_sync)
    }
    async fn send_handshake_req<W: AsyncWrite + std::marker::Unpin>(
        &mut self,
        peer_sink: &mut MessageSink<W, Message>,
    ) -> Result<(), HandShakeError> {
        let handshake_req = HandshakeRequest {
            node_data: self.config.basic_node_data(),
            payload_data: self.get_our_core_sync().await?,
        };
        let message: Message = Message::Request(handshake_req.into());
        peer_sink.send(message).await?;
        Ok(())
    }

    async fn get_handshake_res<R: AsyncRead + std::marker::Unpin>(
        &mut self,
        peer_stream: &mut MessageStream<Message, R>,
    ) -> Result<HandshakeResponse, HandShakeError> {
        // put a timeout on this
        let Message::Response(MessageResponse::Handshake(handshake_res)) =  peer_stream.next().await.expect("MessageSink will not return None")? else {
            return Err(HandShakeError::PeerSentWrongResponse);
        };
        Ok(handshake_res)
    }

    pub async fn complete_handshake<R, W>(
        &mut self,
        peer_reader: R,
        peer_writer: W,
        direction: Direction,
        addr: NetworkAddress,
    ) -> Result<(mpsc::Sender<ClientRequest>, Connection<Svc, W, R>), HandShakeError>
    where
        R: AsyncRead + std::marker::Unpin,
        W: AsyncWrite + std::marker::Unpin,
    {
        let mut peer_sink = MessageSink::new(peer_writer);
        let mut peer_stream = MessageStream::new(peer_reader);
        let (c, conn) = match direction {
            Direction::Outbound => {
                self.send_handshake_req(&mut peer_sink).await?;
                let handshake_res = self.get_handshake_res(&mut peer_stream).await?;
                if !handshake_res
                    .node_data
                    .support_flags
                    .contains(&self.config.minimum_peer_support_flags)
                {
                    return Err(HandShakeError::PeerDoesNotHaveTheMinimumSupportFlags);
                }
                if handshake_res.local_peerlist_new.len() > P2P_MAX_PEERS_IN_HANDSHAKE {
                    return Err(HandShakeError::PeerSentTooManyPeers);
                }

                self.address_book_tx
                    .send(AddressBookUpdate::NewPeers(handshake_res.local_peerlist_new))
                    .await;

                let connection_info = ConnectionInfo { addr };

                let (client_tx, client_rx) = mpsc::channel(2);

                let con = Connection::new(
                    connection_info,
                    peer_sink,
                    peer_stream,
                    client_rx,
                    self.peer_sync_states.clone(),
                    self.peer_request_service.clone(),
                );
                (client_tx, con)
            },
            Direction::Inbound => {
                todo!()
            },
        };

        Ok((c, conn))
    }
}
