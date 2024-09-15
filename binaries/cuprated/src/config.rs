//! cuprated config
use std::time::Duration;

use cuprate_blockchain::config::{
    Config as BlockchainConfig, ConfigBuilder as BlockchainConfigBuilder,
};
use cuprate_consensus::ContextConfig;
use cuprate_p2p::{block_downloader::BlockDownloaderConfig, AddressBookConfig, P2PConfig};
use cuprate_p2p_core::{ClearNet, Network};

pub fn config() -> CupratedConfig {
    // TODO: read config options from the conf files & cli args.

    CupratedConfig {}
}

pub struct CupratedConfig {
    // TODO: expose config options we want to allow changing.
}

impl CupratedConfig {
    pub fn blockchain_config(&self) -> BlockchainConfig {
        BlockchainConfigBuilder::new().fast().build()
    }

    pub fn clearnet_config(&self) -> P2PConfig<ClearNet> {
        P2PConfig {
            network: Network::Mainnet,
            outbound_connections: 16,
            extra_outbound_connections: 0,
            max_inbound_connections: 0,
            gray_peers_percent: 0.7,
            server_config: None,
            p2p_port: 0,
            rpc_port: 0,
            address_book_config: AddressBookConfig {
                max_white_list_length: 1000,
                max_gray_list_length: 5000,
                peer_store_file: "p2p_state.bin".into(),
                peer_save_period: Duration::from_secs(60),
            },
        }
    }

    pub fn block_downloader_config(&self) -> BlockDownloaderConfig {
        BlockDownloaderConfig {
            buffer_size: 50_000_000,
            in_progress_queue_size: 50_000_000,
            check_client_pool_interval: Duration::from_secs(45),
            target_batch_size: 10_000_000,
            initial_batch_size: 1,
        }
    }

    pub fn network(&self) -> Network {
        Network::Mainnet
    }

    pub fn context_config(&self) -> ContextConfig {
        ContextConfig::main_net()
    }
}
