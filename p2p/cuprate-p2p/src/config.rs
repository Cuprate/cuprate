use monero_address_book::AddressBookConfig;

/// P2P config.
pub struct P2PConfig {
    /// The number of outbound connections to make and try keep.
    outbound_connections: usize,
    /// The absolute maximum number of outbound connections.
    ///
    /// *Note:* under rare circumstances Cuprate may make more connections than this number
    /// when checking if peers addresses are reachable. These connections are not held for long
    /// though.
    max_outbound_connections: usize,

    /// The number of anchor connections to make.
    ///
    /// An anchor connection is a connection which was held before last shutdown, anchor connections
    /// help to prevent certain attacks.
    anchor_connections: usize,

    /// The percent of outbound peers that should be gray aka never connected to before.
    ///
    /// Only values 0..=1 are valid.
    gray_peers_percent: f32,

    /// The maximum amount of inbound peers
    max_inbound_connections: usize,

    address_book_config: AddressBookConfig,
}
