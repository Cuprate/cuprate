use std::collections::HashMap;
use std::str::FromStr;

use cuprate_net::levin::{InternalStateMachine, ISMOutput, Direction};
use cuprate_net::NetworkAddress;
use cuprate_net::P2pCommand;
use cuprate_net::messages::BasicNodeData;
use cuprate_net::messages::CoreSyncData;
use cuprate_net::messages::MessageRequest;
use cuprate_net::messages::MessageResponse;
use cuprate_net::messages::MessageNotification;
use cuprate_net::messages::PeerID;
use cuprate_net::messages::admin::HandshakeRequest;
use cuprate_net::messages::admin::HandshakeResponse;

mod outbox;
mod peer_mgr;
pub mod peer_list_mgr;

#[derive(Debug)]
pub enum DisconnectReason {
    PeerSentUnexpectedHandshake,
    PeerIsAlreadyConnected,
}

pub enum Network {
    MainNet,
    StageNet,
    TestNet
}

impl Network {
    pub fn network_id(&self) -> [u8; 16] {
        match self {
            Network::MainNet => [0x12 ,0x30, 0xF1, 0x71 , 0x61, 0x04 , 0x41, 0x61, 0x17, 0x31, 0x00, 0x82, 0x16, 0xA1, 0xA1, 0x10], // Bender's nightmare
            Network::TestNet => [0x12 ,0x30, 0xF1, 0x71 , 0x61, 0x04 , 0x41, 0x61, 0x17, 0x31, 0x00, 0x82, 0x16, 0xA1, 0xA1, 0x11], // Bender's daydream
            Network::StageNet =>[0x12 ,0x30, 0xF1, 0x71 , 0x61, 0x04 , 0x41, 0x61, 0x17, 0x31, 0x00, 0x82, 0x16, 0xA1, 0xA1, 0x12], // Bender's daydream
        }
    }
}

pub struct NodeInfo {
    pub my_port: u32,
    pub network: Network,
    pub peer_id: PeerID,
    pub support_flags: u32,
    pub rpc_port: u16,
    pub rpc_credits_per_hash: u32,
}

impl NodeInfo {
    pub fn basic_node_data(&self) -> BasicNodeData {
        BasicNodeData { 
            my_port: self.my_port, 
            network_id: self.network.network_id(), 
            peer_id: self.peer_id, 
            support_flags: self.support_flags, 
            rpc_port: self.rpc_port, 
            rpc_credits_per_hash: self.rpc_credits_per_hash 
        }
    }
}

pub struct StateMachine<S> {
    node_info: NodeInfo,
    outbox: outbox::OutBox,
    peer_mgr: peer_mgr::PeerMgr,
    peer_list: peer_list_mgr::PeerList<S>,
}

impl<S: peer_list_mgr::PeerStore> StateMachine<S> {
    pub fn new(info: NodeInfo, rng: fastrand::Rng, store: S, peer_store_cfg: peer_list_mgr::PeerStoreConfig) -> StateMachine<S> {
        StateMachine {
            node_info: info,
            outbox: outbox::OutBox::new(),
            peer_mgr: peer_mgr::PeerMgr::new(),
            peer_list: peer_list_mgr::PeerList::new(store, peer_store_cfg, rng)
        }
    }

    fn handle_req(&mut self, addr: NetworkAddress, req: MessageRequest) {
        println!("new req");
        match req {
            MessageRequest::Handshake(handsk) => self.handle_handshake_req(addr, handsk),
            _ => todo!("handle other reqs")
        }
    }

    fn handle_handshake_req(&mut self, addr: NetworkAddress, handsk: HandshakeRequest) {
        self.peer_mgr.handle_handshake_req(addr, &handsk, &mut self.outbox)
    }

}

impl<S: peer_list_mgr::PeerStore> InternalStateMachine for StateMachine<S> {
    type BodyNotification = MessageNotification;
    type BodyRequest = MessageRequest;
    type BodyResponse = MessageResponse;
    type PeerID = NetworkAddress;
    type DisconnectReason = DisconnectReason;

    fn connected(&mut self, addr: &Self::PeerID, direction: Direction) {
        self.peer_mgr.connected(*addr, direction, &mut self.outbox);
    }

    fn disconnected(&mut self, addr: &Self::PeerID) {
        
    }

    fn tick(&mut self) {
        
    }

    fn wake(&mut self) {
        
    }

    fn received_request(&mut self, addr: &Self::PeerID, body: Self::BodyRequest) {
        self.handle_req(*addr, body);
    }

    fn received_response(&mut self, addr: &Self::PeerID, body: Self::BodyResponse) {
        
    }

    fn received_notification(&mut self, addr: &Self::PeerID, body: Self::BodyNotification) {
        
    }

    fn error_decoding_bucket(&mut self, error: cuprate_net::levin::BucketError) {
        
    }
}


impl<S: peer_list_mgr::PeerStore> Iterator for StateMachine<S> {
    type Item = ISMOutput<
    NetworkAddress,
    DisconnectReason,
    MessageNotification,
    MessageRequest,
    MessageResponse,
>;
    fn next(&mut self) -> Option<Self::Item> {
        let Some(output) = self.outbox.next() else {
            return None;
        };
        Some(match output {
            outbox::Event::Internal(_) => todo!("internal event"),
            outbox::Event::Connect(addr) => ISMOutput::Connect(addr),
            outbox::Event::Disconnect(addr, reason) => ISMOutput::Disconnect(addr, reason),
            outbox::Event::SetTimer(timer) => ISMOutput::SetTimer(std::time::Duration::from_secs(timer)),
            outbox::Event::SendRes(addr, command) => ISMOutput::WriteResponse(addr, self.build_message_response(command)),
            e => todo!("{:?}",e)

        })
    }
}

// ******************************************
// **          Message Building            **
// ******************************************

impl<S: peer_list_mgr::PeerStore> StateMachine<S> {

    fn build_message_response(&mut self, command: P2pCommand) -> MessageResponse {
        match command {
            P2pCommand::Handshake => MessageResponse::Handshake(self.build_handshake_response()),
            _ => todo!("msg response")
        }
    }

    fn build_handshake_response(&self) -> HandshakeResponse {
        let peers = self.peer_list.get_peers_for_handshake();
        HandshakeResponse { 
            node_data: self.node_info.basic_node_data(), 
            payload_data: core_sync_data(), 
            local_peerlist_new: peers 
        }
    }
}


fn core_sync_data() -> CoreSyncData {
    CoreSyncData { 
        cumulative_difficulty: 2577619515,
                    cumulative_difficulty_top64: 0,
                    current_height: 12990,
                    pruning_seed: 0,
                    top_id: monero::Hash::from_str("0xe628ba2d4f4fe0a475855482b95e0401397e4a91eeee7a2cd65581ede60e43da").unwrap(),
                    top_version: 1,
    }
}