use cuprate_helper::network::Network;
use monero_address_book::AddressBookConfig;

/// P2P config.
#[derive(Clone, Debug)]
pub struct P2PConfig {
    pub p2p_port: u16,

    pub rpc_port: u16,

    pub network: Network,

    /// The number of outbound connections to make and try keep.
    pub outbound_connections: usize,
    /// The absolute maximum number of held outbound connections.
    ///
    /// *Note:* Cuprate might make more connections than this to see if a peer is reachable or
    /// to get peers from that node, these connections are not held for long though.
    pub max_outbound_connections: usize,

    /// The number of anchor connections to make.
    ///
    /// An anchor connection is a connection which was held before last shutdown, anchor connections
    /// help to prevent certain attacks.
    pub anchor_connections: usize,

    /// The percent of outbound peers that should be gray aka never connected to before.
    ///
    /// Only values 0..=1 are valid.
    pub gray_peers_percent: f64,

    /// The maximum amount of inbound peers
    pub max_inbound_connections: usize,

    pub address_book_config: AddressBookConfig,
}

impl P2PConfig {
    pub fn allowed_extra_connections(&self) -> usize {
        self.max_outbound_connections - self.outbound_connections
    }
}
