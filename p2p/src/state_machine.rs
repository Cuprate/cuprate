use std::collections::HashMap;
use std::str::FromStr;

use cuprate_net::BucketBody;
use cuprate_net::NetworkAddress;
use cuprate_net::BucketStream;
use cuprate_net::bucket::Bucket;
use cuprate_net::bucket::header::P2pCommand;
use cuprate_net::messages::BasicNodeData;
use cuprate_net::messages::CoreSyncData;
use cuprate_net::messages::MessageRequest;
use cuprate_net::messages::MessageResponse;
use cuprate_net::messages::PeerID;
use cuprate_net::messages::admin::HandshakeRequest;
use cuprate_net::messages::admin::HandshakeResponse;

use self::outbox::MsgType;

mod outbox;
mod peer_mgr;
pub mod peer_list_mgr;


#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    Inbound,
    Outbound
}

#[derive(Debug)]
pub enum DisconnectReason {
    PeerSentUnexpectedHandshake,
    PeerIsAlreadyConnected,
}

#[derive(Debug)]
pub enum Output {
    Write(NetworkAddress, Vec<u8>),
    SetTimer(chrono::Duration),
    Disconnect(NetworkAddress, DisconnectReason),
    Connect(NetworkAddress)
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
    streams: HashMap<NetworkAddress, BucketStream>,
    peer_mgr: peer_mgr::PeerMgr,
    peer_list: peer_list_mgr::PeerList<S>,
}

impl<S: peer_list_mgr::PeerStore> StateMachine<S> {
    pub fn new(info: NodeInfo, rng: fastrand::Rng, store: S, peer_store_cfg: peer_list_mgr::PeerStoreConfig) -> StateMachine<S> {
        StateMachine {
            node_info: info,
            outbox: outbox::OutBox::new(),
            streams: HashMap::new(),
            peer_mgr: peer_mgr::PeerMgr::new(),
            peer_list: peer_list_mgr::PeerList::new(store, peer_store_cfg, rng)
        }
    }

    pub fn connected(&mut self, addr: &NetworkAddress, direction: Direction) {
        let stream = BucketStream::default();
        self.streams.insert(*addr, stream);
        self.peer_mgr.connected(*addr, direction, &mut self.outbox);
    }

    pub fn message_received(&mut self, addr: &NetworkAddress, bytes: &[u8]) {
        println!("messaged recived len:{}", bytes.len());
        let Some(stream) = self.streams.get_mut(&addr.clone()) else {
            println!("peer not connected");
            return;
        };
        stream.received_bytes(bytes);
        let mut messages = vec![];
        while let Some(message) = stream.try_decode_next_bucket().unwrap() {////////////////////////
            println!("{:#?}", message);
            messages.push(message)
        }

        for message in messages {
            self.handle_bucket(*addr, message);
        }
    }

    fn handle_bucket(&mut self, addr: NetworkAddress, bucket: Bucket) {
        match bucket.body {
            cuprate_net::BucketBody::Request(req) => self.handle_req(addr, *req),
            _ => todo!("handle res/noti")
        }
    }

    fn handle_req(&mut self, addr: NetworkAddress, req: MessageRequest) {
        match req {
            MessageRequest::Handshake(handsk) => self.handle_handshake_req(addr, handsk),
            _ => todo!("handle other reqs")
        }
    }

    fn handle_handshake_req(&mut self, addr: NetworkAddress, handsk: HandshakeRequest) {
        self.peer_mgr.handle_handshake_req(addr, &handsk, &mut self.outbox)
    }

}

impl<S: peer_list_mgr::PeerStore> Iterator for StateMachine<S> {
    type Item = Output;
    fn next(&mut self) -> Option<Self::Item> {
        let Some(output) = self.outbox.next() else {
            return None;
        };
        Some(match output {
            outbox::Event::Internal(_) => todo!("internal event"),
            outbox::Event::Connect(addr) => Output::Connect(addr),
            outbox::Event::Disconnect(addr, reason) => Output::Disconnect(addr, reason),
            outbox::Event::SendMsg(addr, command , ty) => Output::Write(addr, self.build_bucket_body(command, ty).build_full_bucket_bytes()),
            outbox::Event::SetTimer(timer) => Output::SetTimer(chrono::Duration::seconds(timer)),

        })
    }
}

impl<S: peer_list_mgr::PeerStore> StateMachine<S> {
    fn build_bucket_body(&mut self, command: P2pCommand, ty: MsgType) -> BucketBody {
        match ty {
            MsgType::Response => BucketBody::Response(Box::new(self.build_message_response(command))),
            _ => todo!("bucket")
        }
    }

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