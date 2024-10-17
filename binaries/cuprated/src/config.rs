//! cuprated config

use cuprate_consensus::ContextConfig;
use cuprate_helper::network::Network;
use cuprate_p2p::block_downloader::BlockDownloaderConfig;
use cuprate_p2p_core::ClearNet;
use serde::{Deserialize, Serialize};
use std::time::Duration;

mod p2p;
mod storage;

use p2p::P2PConfig;
use storage::StorageConfig;

pub fn config() -> Config {
    Config::default()
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    network: Network,

    p2p: P2PConfig,

    storage: StorageConfig,
}

impl Config {
    pub fn network(&self) -> Network {
        self.network
    }

    pub fn clearnet_p2p_config(&self) -> cuprate_p2p::P2PConfig<ClearNet> {
        cuprate_p2p::P2PConfig {
            network: self.network,
            outbound_connections: self.p2p.clear_net.general.outbound_connections,
            extra_outbound_connections: self.p2p.clear_net.general.extra_outbound_connections,
            max_inbound_connections: self.p2p.clear_net.general.max_inbound_connections,
            gray_peers_percent: self.p2p.clear_net.general.gray_peers_percent,
            server_config: Some(self.p2p.clear_net.server.clone()),
            p2p_port: self.p2p.clear_net.general.p2p_port,
            rpc_port: 0,
            address_book_config: self.p2p.clear_net.general.address_book_config.clone(),
        }
    }

    pub fn context_config(&self) -> ContextConfig {
        match self.network {
            Network::Mainnet => ContextConfig::main_net(),
            Network::Stagenet => ContextConfig::stage_net(),
            Network::Testnet => ContextConfig::test_net(),
        }
    }

    pub fn blockchain_config(&self) -> cuprate_blockchain::config::Config {
        self.storage.blockchain.clone()
    }

    pub fn block_downloader_config(&self) -> BlockDownloaderConfig {
        BlockDownloaderConfig {
            buffer_size: 50_000_000,
            in_progress_queue_size: 50_000_000,
            check_client_pool_interval: Duration::from_secs(30),
            target_batch_size: 5_000_000,
            initial_batch_size: 1,
        }
    }
}
