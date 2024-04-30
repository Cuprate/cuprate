/// P2P config.
#[derive(Clone, Debug)]
pub struct P2PConfig {
    /// The number of outbound connections to make and try keep.
    pub outbound_connections: usize,
    /// The absolute maximum number of held outbound connections.
    ///
    /// *Note:* Cuprate might make more connections than this to see if a peer is reachable or
    /// to get peers from that node, these connections are not held for long though.
    pub max_outbound_connections: usize,

    /// The percent of outbound peers that should be gray aka never connected to before.
    ///
    /// Only values 0..=1 are valid.
    pub gray_peers_percent: f64,
}

impl P2PConfig {
    pub fn allowed_extra_connections(&self) -> usize {
        self.max_outbound_connections - self.outbound_connections
    }
}
