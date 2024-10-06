use cuprate_address_book::AddressBookConfig;
use cuprate_p2p_core::ClearNetServerCfg;
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct P2PConfig {
    pub clear_net: ClearNetConfig,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ClearNetConfig {
    pub server: ClearNetServerCfg,
    #[serde(flatten)]
    pub general: SharedNetConfig,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct SharedNetConfig {
    /// The number of outbound connections to make and try keep.
    pub outbound_connections: usize,
    /// The amount of extra connections we can make if we are under load from the rest of Cuprate.
    pub extra_outbound_connections: usize,
    /// The maximum amount of inbound connections
    pub max_inbound_connections: usize,
    pub gray_peers_percent: f64,
    /// port to use to accept p2p connections.
    pub p2p_port: u16,
    /// The address book config.
    pub address_book_config: AddressBookConfig,
}

impl Default for SharedNetConfig {
    fn default() -> Self {
        Self {
            outbound_connections: 32,
            extra_outbound_connections: 8,
            max_inbound_connections: 128,
            gray_peers_percent: 0.7,
            p2p_port: 18080,
            address_book_config: AddressBookConfig::default(),
        }
    }
}
