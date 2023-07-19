use cuprate_common::Network;
use monero_wire::messages::{common::PeerSupportFlags, BasicNodeData, PeerID};

use crate::{
    constants::{
        CUPRATE_SUPPORT_FLAGS, DEFAULT_IN_PEERS, DEFAULT_LOAD_OUT_PEERS_MULTIPLIER,
        DEFAULT_TARGET_OUT_PEERS, MAX_GRAY_LIST_PEERS, MAX_WHITE_LIST_PEERS,
    },
    NodeID,
};

#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Port
    my_port: u32,
    /// The Network
    network: Network,
    /// RPC Port
    rpc_port: u16,

    target_out_peers: usize,
    out_peers_load_multiplier: usize,
    max_in_peers: usize,
    max_white_peers: usize,
    max_gray_peers: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            my_port: 18080,
            network: Network::MainNet,
            rpc_port: 18081,
            target_out_peers: DEFAULT_TARGET_OUT_PEERS,
            out_peers_load_multiplier: DEFAULT_LOAD_OUT_PEERS_MULTIPLIER,
            max_in_peers: DEFAULT_IN_PEERS,
            max_white_peers: MAX_WHITE_LIST_PEERS,
            max_gray_peers: MAX_GRAY_LIST_PEERS,
        }
    }
}

impl Config {
    pub fn basic_node_data(&self, peer_id: PeerID) -> BasicNodeData {
        BasicNodeData {
            my_port: self.my_port,
            network_id: self.network.network_id(),
            peer_id,
            support_flags: CUPRATE_SUPPORT_FLAGS,
            rpc_port: self.rpc_port,
            rpc_credits_per_hash: 0,
        }
    }

    pub fn peerset_total_connection_limit(&self) -> usize {
        self.target_out_peers * self.out_peers_load_multiplier + self.max_in_peers
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub fn max_white_peers(&self) -> usize {
        self.max_white_peers
    }

    pub fn max_gray_peers(&self) -> usize {
        self.max_gray_peers
    }

    pub fn public_port(&self) -> u32 {
        self.my_port
    }

    pub fn public_rpc_port(&self) -> u16 {
        self.rpc_port
    }
}
