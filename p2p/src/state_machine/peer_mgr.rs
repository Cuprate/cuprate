use std::collections::{HashMap};

use cuprate_net::{messages::{admin::HandshakeRequest}, NetworkAddress};
use chrono::{DateTime, Utc};

use super::outbox::OutBox;
use super::{DisconnectReason, Direction};

struct Peer {
    pub port: u32,
    pub support_flags: u32,
    pub rpc_port: u16,
    pub rpc_credits_per_hash: u32,
    pub last_seen: DateTime<Utc>,
    pub pruning_seed: u32,
}


pub struct PeerMgr {
    connected: HashMap<NetworkAddress, Peer>,
    pre_handshake_peers: HashMap<NetworkAddress, DateTime<Utc>>,
    waiting_for_response_peers: HashMap<NetworkAddress, DateTime<Utc>>,
    inbound: usize,
    outbound: usize,
    grey: usize,
    white: usize

}

impl PeerMgr {
    pub fn new() -> PeerMgr {
        PeerMgr { 
            connected: HashMap::new(), 
            pre_handshake_peers: HashMap::new(), 
            waiting_for_response_peers: HashMap::new(), 
            inbound: 0, 
            outbound: 0, 
            grey: 0, 
            white: 0 
        }
    }

    fn tick(&mut self, outbox: &mut OutBox) {
        todo!("tick peermgr")
    }

    pub fn connected(&mut self, addr: NetworkAddress, direction: Direction, outbox: &mut OutBox) {
        // outbound connections should have already been added to the corresponding map
        if direction == Direction::Inbound {
            self.pre_handshake_peers.insert(addr, Utc::now());
        }
    }

    pub fn handle_handshake_req(&mut self, addr: NetworkAddress, req: &HandshakeRequest, outbox: &mut OutBox) {
        if self.connected.contains_key(&addr) || self.waiting_for_response_peers.contains_key(&addr) {
            outbox.disconnect(addr, DisconnectReason::PeerSentUnexpectedHandshake);
            return;
        }
        let peer = Peer {
            port: req.node_data.my_port,
            support_flags: req.node_data.support_flags,
            rpc_port: req.node_data.rpc_port,
            rpc_credits_per_hash: req.node_data.rpc_credits_per_hash,
            last_seen: Utc::now(),
            pruning_seed: req.payload_data.pruning_seed,
        };

        let Some(_) = self.pre_handshake_peers.remove(&addr) else {
            outbox.disconnect(addr, DisconnectReason::PeerSentUnexpectedHandshake);
            return;
        };

        self.connected.insert(addr.clone(), peer);

        outbox.send_handshake_response(addr)

    }
}