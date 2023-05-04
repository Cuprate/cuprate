pub mod address_book;
pub mod config;
pub mod connection_counter;
mod constants;
pub mod peer;
pub mod peer_set;
mod protocol;

pub use config::Config;

pub enum NodeID {
    Public(monero_wire::PeerID),
    Tor(monero_wire::PeerID),
    I2p(monero_wire::PeerID),
}

#[async_trait::async_trait]
pub trait P2PStore: Clone + Send + 'static {
    /// Loads the peers from the peer store.
    /// returns (in order):
    /// the white list,
    /// the gray list,
    /// the anchor list,
    /// the ban list
    async fn load_peers(
        &mut self,
        zone: monero_wire::NetZone,
    ) -> Result<
        (
            Vec<monero_wire::PeerListEntryBase>, // white list
            Vec<monero_wire::PeerListEntryBase>, // gray list
            Vec<monero_wire::NetworkAddress>,    // anchor list
            Vec<(monero_wire::NetworkAddress, chrono::NaiveDateTime)>, // ban list
        ),
        &'static str,
    >;

    async fn save_peers(
        &mut self,
        zone: monero_wire::NetZone,
        white: Vec<&monero_wire::PeerListEntryBase>,
        gray: Vec<&monero_wire::PeerListEntryBase>,
        anchor: Vec<&monero_wire::NetworkAddress>,
        bans: Vec<(&monero_wire::NetworkAddress, &chrono::NaiveDateTime)>, // ban lists
    ) -> Result<(), &'static str>;

    async fn node_id(&mut self) -> Result<Option<NodeID>, &'static str>;

    async fn save_node_id(&mut self, node_id: NodeID) -> Result<(), &'static str>;
}
