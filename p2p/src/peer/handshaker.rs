use futures::{channel::mpsc, AsyncWrite, AsyncRead, SinkExt, StreamExt};
use monero_wire::{messages::{common::PeerSupportFlags, PeerListEntryBase, admin::{HandshakeRequest, HandshakeResponse}, PeerID, BasicNodeData, CoreSyncData, MessageResponse}, NetworkAddress, levin::{MessageSink, MessageStream, BucketError}, Message};
use thiserror::Error;
use tower::{Service, ServiceExt};

use crate::{Network, protocol::{InternalMessageRequest, InternalMessageResponse}};

use super::{client::Client, Direction, connection::{PeerInfo, Connection}, P2P_MAX_PEERS_IN_HANDSHAKE, RequestServiceError};

#[derive(Debug, Error)]
pub enum HandShakeError {
    #[error("The peer does not have the minimum support flags")]
    PeerDoesNotHaveTheMinimumSupportFlags,
    #[error("The address book channel has closed")]
    AddressBookChannelClosed,
    #[error("The peer sent too many peers, considered spamming")]
    PeerSentTooManyPeers,
    #[error("The peer sent a wrong response to our handshake")]
    PeerSentWrongResponse,
    #[error("The syncer returned an error")]
    SyncerError,
    #[error("Bucket error while communicating with peer: {0}")]
    BucketError(#[from] BucketError),
}

pub enum AddressBookUpdate {
    NewPeers(Vec<PeerListEntryBase>),
    WhiteList(NetworkAddress),
    RemovePeer(NetworkAddress),
    BanPeer(NetworkAddress),
    AnchorPeer(NetworkAddress)
}

pub enum SyncerRequest {
    CoreSyncData
}

pub enum SyncerResponse {
    CoreSyncData(CoreSyncData),
}

impl SyncerResponse {
    pub fn core_sync_data(self) -> Option<CoreSyncData> {
        match self {
            Self::CoreSyncData(csd)=> Some(csd),
            _ => None
        }
    }
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

impl NetworkConfig {
    pub fn basic_node_data(&self) -> BasicNodeData {
        BasicNodeData { 
            my_port: self.my_port, 
            network_id: self.network.network_id(), 
            peer_id: self.peer_id, 
            support_flags: self.our_support_flags, 
            rpc_port: self.rpc_port, 
            rpc_credits_per_hash: self.rpc_credits_per_hash 
        }
    }
}

pub struct Handshaker<Syc, Isv> {
    config: NetworkConfig,
    address_book_tx: mpsc::Sender<AddressBookUpdate>,
    syncer: Syc, 
    inbound_service: Isv
}

impl<Syc, Isv> Handshaker<Syc, Isv>
where
    Syc: Service<SyncerRequest, Response = SyncerResponse>,
    Isv: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = RequestServiceError>
{
    async fn send_handshake_req<W: AsyncWrite + std::marker::Unpin>(&mut self, peer_sink: &mut MessageSink<W, Message>) -> Result<(), HandShakeError> {
        let syncer = self.syncer.ready().await.map_err(|_| HandShakeError::SyncerError)?;
        let handshake_req = HandshakeRequest{
            node_data: self.config.basic_node_data(), 
            payload_data: syncer.call(
                SyncerRequest::CoreSyncData)
                .await
                .map_err(|_| HandShakeError::SyncerError)?
                .core_sync_data()
                .expect("The syncer should return what we asked for")
        };
        let message: Message = Message::Request(handshake_req.into());
        peer_sink.send(message).await?;
        Ok(())
    }
    async fn get_handshake_res<R: AsyncRead + std::marker::Unpin>(&mut self, peer_stream: &mut MessageStream<Message, R>) -> Result<HandshakeResponse, HandShakeError> {
        // put a timeout on this
        let Message::Response(MessageResponse::Handshake(handshake_res)) =  peer_stream.next().await.expect("MessageSink will not return None")? else {
            return Err(HandShakeError::PeerSentWrongResponse);
        };
        Ok(handshake_res)
    }

    fn build_peer_info_from_nd_csd(&self, node_data: &BasicNodeData, payload_data: &CoreSyncData, direction: Direction) -> PeerInfo {
        PeerInfo {
            id: node_data.peer_id,
            port: node_data.my_port,
            current_height: payload_data.current_height,
            cumulative_difficulty: payload_data.cumulative_difficulty(),
            support_flags: node_data.support_flags,
            pruning_seed: payload_data.pruning_seed,
            rpc_port: node_data.rpc_port,
            rpc_credits_per_hash: node_data.rpc_credits_per_hash,
            network: self.config.network,
            direction,
        }
    }

    

    pub async fn complete_handshake<R, W, Svc>(&mut self, peer_reader: R, peer_writer: W, direction: Direction) -> Result<(Client, Connection<Svc, W, R>), HandShakeError>
    where
        R: AsyncRead + std::marker::Unpin,
        W: AsyncWrite + std::marker::Unpin,
    {
        let mut peer_sink = MessageSink::new(peer_writer);
        let mut peer_stream =  MessageStream::new(peer_reader);
        match direction {
            Direction::Outbound => {
                self.send_handshake_req(&mut peer_sink).await?;
                let handshake_res = self.get_handshake_res(&mut peer_stream).await?;
                if !handshake_res.node_data.support_flags.contains(&self.config.minimum_peer_support_flags) {
                    return Err(HandShakeError::PeerDoesNotHaveTheMinimumSupportFlags);
                }
                if handshake_res.local_peerlist_new.len() > P2P_MAX_PEERS_IN_HANDSHAKE {
                    return Err(HandShakeError::PeerSentTooManyPeers);
                }
                self.address_book_tx.send(AddressBookUpdate::NewPeers(handshake_res.local_peerlist_new)).await;
                let peer_info = self.build_peer_info_from_nd_csd(&handshake_res.node_data, &handshake_res.payload_data, direction);


            }
        }
        Ok(())
    }
}
