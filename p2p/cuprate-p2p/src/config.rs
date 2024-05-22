/// P2P config.
#[derive(Clone, Debug)]
pub struct P2PConfig {
    /// The number of outbound connections to make and try keep.
    pub outbound_connections: usize,
    /// The amount of extra connections we can make if we are under load from the rest of Cuprate.
    pub extra_outbound_connections: usize,
    /// The percent of outbound peers that should be gray aka never connected to before.
    ///
    /// Only values 0..=1 are valid.
    pub gray_peers_percent: f64,
}
