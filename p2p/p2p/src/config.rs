use cuprate_helper::network::Network;
use cuprate_p2p_core::NetworkZone;
use cuprate_wire::{common::PeerSupportFlags, BasicNodeData};

pub use cuprate_address_book::AddressBookConfig;

/// P2P config.
#[derive(Clone, Debug)]
pub struct P2PConfig<N: NetworkZone> {
    /// The [`Network`] we should connect to.
    pub network: Network,
    /// Seed nodes to connect to find peers if our address book is empty.
    pub seeds: Vec<N::Addr>,

    /// The number of outbound connections to make and try keep.
    pub outbound_connections: usize,
    /// The amount of extra connections we can make if we are under load from the rest of Cuprate.
    pub extra_outbound_connections: usize,
    /// The maximum amount of inbound connections, only relevant if [`P2PConfig::server_config`] is set to [`Some`]
    pub max_inbound_connections: usize,
    /// The percent of outbound peers that should be gray aka never connected to before.
    ///
    /// Only values 0..=1 are valid.
    pub gray_peers_percent: f64,
    /// The inbound server configuration,
    ///
    /// If this is [`None`] no inbound connections will be accepted.
    pub server_config: Option<N::ServerCfg>,

    /// The port to listen on for inbound connections, only relevant if [`P2PConfig::server_config`] is set to [`Some`].
    pub p2p_port: u16,
    /// The public RPC port to tell peers about so wallets can use our node. `0` if we do not have a public RPC port.
    pub rpc_port: u16,

    /// The [`AddressBookConfig`].
    pub address_book_config: AddressBookConfig,
}

impl<N: NetworkZone> P2PConfig<N> {
    /// Returns the [`BasicNodeData`] for this [`P2PConfig`].
    ///
    /// [`BasicNodeData::peer_id`] is set to a random u64, so this function should only be called once
    /// per [`NetworkZone`] per run.
    pub(crate) fn basic_node_data(&self) -> BasicNodeData {
        BasicNodeData {
            my_port: u32::from(self.p2p_port),
            network_id: self.network.network_id(),
            peer_id: rand::random(),
            support_flags: PeerSupportFlags::FLUFFY_BLOCKS,
            rpc_port: self.rpc_port,
            // We do not (and probably will never) support paying for RPC with hashes.
            rpc_credits_per_hash: 0,
        }
    }
}
