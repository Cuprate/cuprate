pub mod address_book;
pub mod config;
pub mod connection_counter;
mod connection_handle;
mod constants;
pub mod peer;
mod protocol;

pub use config::Config;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct NetZoneBasicNodeData {
    public: monero_wire::BasicNodeData,
    tor: monero_wire::BasicNodeData,
    i2p: monero_wire::BasicNodeData,
}

impl NetZoneBasicNodeData {
    pub fn basic_node_data(&self, net_zone: &monero_wire::NetZone) -> monero_wire::BasicNodeData {
        match net_zone {
            monero_wire::NetZone::Public => self.public.clone(),
            _ => todo!(),
        }
    }
    pub fn new(config: &Config, node_id: &NodeID) -> Self {
        let bnd = monero_wire::BasicNodeData {
            my_port: config.public_port(),
            network_id: config.network().network_id(),
            peer_id: node_id.public,
            support_flags: constants::CUPRATE_SUPPORT_FLAGS,
            rpc_port: config.public_rpc_port(),
            rpc_credits_per_hash: 0,
        };

        // obviously this is wrong, i will change when i add tor support
        NetZoneBasicNodeData {
            public: bnd.clone(),
            tor: bnd.clone(),
            i2p: bnd,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeID {
    public: monero_wire::PeerID,
    tor: monero_wire::PeerID,
    i2p: monero_wire::PeerID,
}

impl NodeID {
    pub fn generate() -> NodeID {
        let mut rng = rand::thread_rng();
        NodeID {
            public: monero_wire::PeerID(rng.gen()),
            tor: monero_wire::PeerID(rng.gen()),
            i2p: monero_wire::PeerID(rng.gen()),
        }
    }
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
                                                 // Vec<(monero_wire::NetworkAddress, chrono::NaiveDateTime)>, // ban list
        ),
        &'static str,
    >;

    async fn save_peers(
        &mut self,
        zone: monero_wire::NetZone,
        white: Vec<&monero_wire::PeerListEntryBase>,
        gray: Vec<&monero_wire::PeerListEntryBase>,
        anchor: Vec<&monero_wire::NetworkAddress>,
        //  bans: Vec<(&monero_wire::NetworkAddress, &chrono::NaiveDateTime)>, // ban lists
    ) -> Result<(), &'static str>;

    async fn basic_node_data(&mut self) -> Result<Option<NetZoneBasicNodeData>, &'static str>;

    async fn save_basic_node_data(
        &mut self,
        node_id: &NetZoneBasicNodeData,
    ) -> Result<(), &'static str>;
}
