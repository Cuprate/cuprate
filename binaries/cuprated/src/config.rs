//! cuprated config
use std::{
    fs::{read_to_string, File},
    io,
    path::Path,
    time::Duration,
};

use clap::Parser;
use serde::{Deserialize, Serialize};

use cuprate_consensus::ContextConfig;
use cuprate_helper::{
    fs::{CUPRATE_CONFIG_DIR, DEFAULT_CONFIG_FILE_NAME},
    network::Network,
};
use cuprate_p2p::block_downloader::BlockDownloaderConfig;
use cuprate_p2p_core::{ClearNet, ClearNetServerCfg};

use crate::{
    constants::{DEFAULT_CONFIG_STARTUP_DELAY, DEFAULT_CONFIG_WARNING},
    logging::eprintln_red,
};

mod args;
mod fs;
mod p2p;
mod rayon;
mod rpc;
mod storage;
mod tokio;
mod tracing_config;

use fs::FileSystemConfig;
use p2p::P2PConfig;
use rayon::RayonConfig;
pub use rpc::RpcConfig;
use storage::StorageConfig;
use tokio::TokioConfig;
use tracing_config::TracingConfig;

/// Reads the args & config file, returning a [`Config`].
pub fn read_config_and_args() -> Config {
    let args = args::Args::parse();
    args.do_quick_requests();

    let config: Config = if let Some(config_file) = &args.config_file {
        // If a config file was set in the args try to read it and exit if we can't.
        match Config::read_from_path(config_file) {
            Ok(config) => config,
            Err(e) => {
                eprintln_red(&format!("Failed to read config from file: {e}"));
                std::process::exit(1);
            }
        }
    } else {
        // First attempt to read the config file from the current directory.
        std::env::current_dir()
            .map(|path| path.join(DEFAULT_CONFIG_FILE_NAME))
            .map_err(Into::into)
            .and_then(Config::read_from_path)
            .inspect_err(|e| tracing::debug!("Failed to read config from current dir: {e}"))
            // otherwise try the main config directory.
            .or_else(|_| {
                let file = CUPRATE_CONFIG_DIR.join(DEFAULT_CONFIG_FILE_NAME);
                Config::read_from_path(file)
            })
            .inspect_err(|e| {
                tracing::debug!("Failed to read config from config dir: {e}");
                if !args.skip_config_warning {
                    eprintln_red(DEFAULT_CONFIG_WARNING);
                    std::thread::sleep(DEFAULT_CONFIG_STARTUP_DELAY);
                }
            })
            .unwrap_or_default()
    };

    args.apply_args(config)
}

/// The config for all of Cuprate.
#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// The network we should run on.
    network: Network,

    pub no_fast_sync: bool,

    /// [`tracing`] config.
    pub tracing: TracingConfig,

    pub tokio: TokioConfig,

    pub rayon: RayonConfig,

    /// The P2P network config.
    p2p: P2PConfig,

    /// The RPC config.
    pub rpc: RpcConfig,

    /// The storage config.
    pub storage: StorageConfig,

    pub fs: FileSystemConfig,
}

impl Config {
    /// Attempts to read a config file in [`toml`] format from the given [`Path`].
    ///
    /// # Errors
    ///
    /// Will return an [`Err`] if the file cannot be read or if the file is not a valid [`toml`] config.
    fn read_from_path(file: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let file_text = read_to_string(file.as_ref())?;

        Ok(toml::from_str(&file_text)
            .inspect(|_| println!("Using config at: {}", file.as_ref().to_string_lossy()))
            .inspect_err(|e| {
                eprintln_red(&format!(
                    "Failed to parse config file at: {}",
                    file.as_ref().to_string_lossy()
                ));
                eprintln_red(&format!("{e}"));
                std::process::exit(1);
            })?)
    }

    /// Returns the current [`Network`] we are running on.
    pub const fn network(&self) -> Network {
        self.network
    }

    /// The [`ClearNet`], [`cuprate_p2p::P2PConfig`].
    pub fn clearnet_p2p_config(&self) -> cuprate_p2p::P2PConfig<ClearNet> {
        cuprate_p2p::P2PConfig {
            network: self.network,
            seeds: p2p::clear_net_seed_nodes(self.network),
            outbound_connections: self.p2p.clear_net.general.outbound_connections,
            extra_outbound_connections: self.p2p.clear_net.general.extra_outbound_connections,
            max_inbound_connections: self.p2p.clear_net.general.max_inbound_connections,
            gray_peers_percent: self.p2p.clear_net.general.gray_peers_percent,
            server_config: Some(ClearNetServerCfg {
                ip: self.p2p.clear_net.listen_on,
            }),
            p2p_port: self.p2p.clear_net.general.p2p_port,
            rpc_port: self.rpc.port_restricted().unwrap_or(0),
            address_book_config: self
                .p2p
                .clear_net
                .general
                .address_book_config(&self.fs.cache_directory, self.network),
        }
    }

    /// The [`ContextConfig`].
    pub const fn context_config(&self) -> ContextConfig {
        match self.network {
            Network::Mainnet => ContextConfig::main_net(),
            Network::Stagenet => ContextConfig::stage_net(),
            Network::Testnet => ContextConfig::test_net(),
        }
    }

    /// The [`cuprate_blockchain`] config.
    pub fn blockchain_config(&self) -> cuprate_blockchain::config::Config {
        let blockchain = &self.storage.blockchain;

        // We don't set reader threads as we manually make the reader threadpool.
        cuprate_blockchain::config::ConfigBuilder::default()
            .network(self.network)
            .data_directory(self.fs.data_directory.clone())
            .sync_mode(blockchain.shared.sync_mode)
            .build()
    }

    /// The [`cuprate_txpool`] config.
    pub fn txpool_config(&self) -> cuprate_txpool::config::Config {
        let txpool = &self.storage.txpool;

        // We don't set reader threads as we manually make the reader threadpool.
        cuprate_txpool::config::ConfigBuilder::default()
            .network(self.network)
            .data_directory(self.fs.data_directory.clone())
            .sync_mode(txpool.shared.sync_mode)
            .build()
    }

    /// The [`BlockDownloaderConfig`].
    pub fn block_downloader_config(&self) -> BlockDownloaderConfig {
        self.p2p.block_downloader.clone().into()
    }
}

#[cfg(test)]
mod test {
    use toml::from_str;

    use crate::constants::EXAMPLE_CONFIG;

    use super::*;

    /// Tests the latest config is the `Default`.
    #[test]
    fn config_latest() {
        let config: Config = from_str(EXAMPLE_CONFIG).unwrap();
        assert_eq!(config, Config::default());
    }

    /// Tests backwards compatibility.
    #[test]
    fn config_backwards_compat() {
        // (De)serialization tests.
        #[expect(
            clippy::single_element_loop,
            reason = "Remove after adding other versions"
        )]
        for version in ["0.0.1"] {
            let path = format!("config/{version}.toml");
            println!("Testing config serde backwards compat: {path}");
            let string = read_to_string(path).unwrap();
            from_str::<Config>(&string).unwrap();
        }
    }
}
