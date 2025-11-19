//! cuprated config
use std::{
    fmt,
    fs::{read_to_string, File},
    io,
    net::{IpAddr, TcpListener},
    path::Path,
    str::FromStr,
    time::Duration,
};

use anyhow::bail;
use arti_client::KeystoreSelector;
use clap::Parser;
use safelog::DisplayRedacted;
use serde::{Deserialize, Serialize};

use cuprate_consensus::ContextConfig;
use cuprate_helper::{
    fs::{CUPRATE_CONFIG_DIR, DEFAULT_CONFIG_FILE_NAME},
    network::Network,
};
use cuprate_p2p::block_downloader::BlockDownloaderConfig;
use cuprate_p2p_core::{ClearNet, Tor};
use cuprate_wire::OnionAddr;

use crate::{
    constants::{DEFAULT_CONFIG_STARTUP_DELAY, DEFAULT_CONFIG_WARNING},
    logging::eprintln_red,
    tor::{TorContext, TorMode},
};

mod args;
mod default;
mod fs;
mod p2p;
mod rayon;
mod rpc;
mod storage;
mod tokio;
mod tor;
mod tracing_config;

#[macro_use]
mod macros;

use fs::FileSystemConfig;
pub use p2p::{p2p_port, P2PConfig};
use rayon::RayonConfig;
pub use rpc::{restricted_rpc_port, unrestricted_rpc_port, RpcConfig};
pub use storage::{StorageConfig, TxpoolConfig};
use tokio::TokioConfig;
use tor::TorConfig;
use tracing_config::TracingConfig;

/// Header to put at the start of the generated config file.
const HEADER: &str = r"##     ____                      _
##    / ___|   _ _ __  _ __ __ _| |_ ___
##   | |  | | | | '_ \| '__/ _` | __/ _ \
##   | |__| |_| | |_) | | | (_| | ||  __/
##    \____\__,_| .__/|_|  \__,_|\__\___|
##              |_|
##
## All these config values can be set to
## their default by commenting them out with '#'.
##
## Some values are already commented out,
## to set the value remove the '#' at the start of the line.
##
## For more documentation, see: <https://user.cuprate.org>.

";

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

    let config = args.apply_args(config);

    if args.dry_run {
        config.dry_run_check();
    }

    config
}

config_struct! {
    /// The config for all of Cuprate.
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct Config {
        /// The network cuprated should run on.
        ///
        /// Valid values | "Mainnet", "Testnet", "Stagenet"
        pub network: Network,

        /// Enable/disable fast sync.
        ///
        /// Fast sync skips verification of old blocks by
        /// comparing block hashes to a built-in hash file,
        /// disabling this will significantly increase sync time.
        /// New blocks are still fully validated.
        ///
        /// Type         | boolean
        /// Valid values | true, false
        pub fast_sync: bool,

        #[child = true]
        /// Configuration for cuprated's logging system, tracing.
        ///
        /// Tracing is used for logging to stdout and files.
        pub tracing: TracingConfig,

        #[child = true]
        /// Configuration for cuprated's asynchronous runtime system, tokio.
        ///
        /// Tokio is used for network operations and the major services inside `cuprated`.
        pub tokio: TokioConfig,

        #[child = true]
        /// Configuration for cuprated's thread-pool system, rayon.
        ///
        /// Rayon is used for CPU intensive tasks.
        pub rayon: RayonConfig,

        #[child = true]
        /// Configuration for cuprated's P2P system.
        pub p2p: P2PConfig,

        #[child = true]
        /// Configuration for cuprated's Tor component
        pub tor: TorConfig,

        #[child = true]
        /// Configuration for cuprated's RPC system.
        pub rpc: RpcConfig,

        #[child = true]
        /// Configuration for persistent data storage.
        pub storage: StorageConfig,

        #[child = true]
        /// Configuration for the file-system.
        pub fs: FileSystemConfig,
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: Default::default(),
            fast_sync: true,
            tracing: Default::default(),
            tokio: Default::default(),
            tor: Default::default(),
            rayon: Default::default(),
            p2p: Default::default(),
            rpc: Default::default(),
            storage: Default::default(),
            fs: Default::default(),
        }
    }
}

impl Config {
    /// Returns a default [`Config`], with doc comments.
    pub fn documented_config() -> String {
        let str = toml::ser::to_string_pretty(&Self::default()).unwrap();
        let mut doc = toml_edit::DocumentMut::from_str(&str).unwrap();
        Self::write_docs(doc.as_table_mut());
        format!("{HEADER}{doc}")
    }

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
            outbound_connections: self.p2p.clear_net.outbound_connections,
            extra_outbound_connections: self.p2p.clear_net.extra_outbound_connections,
            max_inbound_connections: self.p2p.clear_net.max_inbound_connections,
            gray_peers_percent: self.p2p.clear_net.gray_peers_percent,
            p2p_port: p2p_port(self.p2p.clear_net.p2p_port, self.network),
            rpc_port: self.rpc.restricted.port_for_p2p(self.network),
            address_book_config: self.p2p.clear_net.address_book_config.address_book_config(
                &self.fs.cache_directory,
                self.network,
                None,
            ),
        }
    }

    /// The [`Tor`], [`cuprate_p2p::P2PConfig`].
    pub fn tor_p2p_config(&self, ctx: &TorContext) -> cuprate_p2p::P2PConfig<Tor> {
        let inbound_enabled = self.p2p.tor_net.inbound_onion;

        let tor_p2p_port = p2p_port(self.p2p.tor_net.p2p_port, self.network);

        let our_onion_address = match ctx.mode {
            TorMode::Off => None,
            TorMode::Daemon => inbound_enabled.then(||
                OnionAddr::new(
                    &self.tor.daemon.anonymous_inbound,
                    tor_p2p_port
                ).expect("Unable to parse supplied `anonymous_inbound` onion address. Please make sure the address is correct.")),
            TorMode::Arti => inbound_enabled.then(|| {
                let addr = ctx.arti_onion_service
                    .as_ref()
                    .unwrap()
                    .generate_identity_key(KeystoreSelector::Primary)
                    .unwrap()
                    .display_unredacted()
                    .to_string();

                OnionAddr::new(&addr, tor_p2p_port).unwrap()
            })
        };

        cuprate_p2p::P2PConfig {
            network: self.network,
            seeds: p2p::tor_net_seed_nodes(self.network),
            outbound_connections: self.p2p.tor_net.outbound_connections,
            extra_outbound_connections: self.p2p.tor_net.extra_outbound_connections,
            max_inbound_connections: self.p2p.tor_net.max_inbound_connections,
            gray_peers_percent: self.p2p.tor_net.gray_peers_percent,
            p2p_port: tor_p2p_port,
            rpc_port: 0,
            address_book_config: self.p2p.tor_net.address_book_config.address_book_config(
                &self.fs.cache_directory,
                self.network,
                our_onion_address,
            ),
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
            .sync_mode(blockchain.sync_mode)
            .build()
    }

    /// The [`cuprate_txpool`] config.
    pub fn txpool_config(&self) -> cuprate_txpool::config::Config {
        let txpool = &self.storage.txpool;

        // We don't set reader threads as we manually make the reader threadpool.
        cuprate_txpool::config::ConfigBuilder::default()
            .network(self.network)
            .data_directory(self.fs.data_directory.clone())
            .sync_mode(txpool.sync_mode)
            .build()
    }

    /// The [`BlockDownloaderConfig`].
    pub fn block_downloader_config(&self) -> BlockDownloaderConfig {
        self.p2p.block_downloader.clone().into()
    }

    /// Checks if a port can be bound to.
    /// Returns `Ok(())` if the port is available, otherwise returns an error.
    fn check_port(ip: IpAddr, port: u16) -> Result<(), anyhow::Error> {
        match TcpListener::bind((ip, port)) {
            Ok(_) => Ok(()),
            Err(e) => {
                bail!("Failed to bind {ip}:{port} - {e}")
            }
        }
    }

    /// Create directory at path if it doesn't exists.
    /// Checks if directory has proper read/write permissions.
    fn check_dir_permissions(path: &Path) -> Result<(), anyhow::Error> {
        if !path.exists() {
            if let Err(e) = std::fs::create_dir_all(path) {
                bail!("Cannot create directory {}: {e}", path.display());
            }
        }

        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => bail!("Cannot access {}: {e}", path.display()),
        };

        if !metadata.is_dir() {
            bail!("Path {} is not a directory", path.display());
        }

        if let Err(e) = std::fs::read_dir(path) {
            bail!("No read permission for {}", path.display())
        }

        let test_file = path.join(".cuprate_write_test");
        if let Err(e) = std::fs::write(&test_file, b"Cuprate") {
            bail!("No write permission for {}", path.display());
        }

        if let Err(e) = std::fs::remove_file(&test_file) {
            bail!("Cannot remove temporary file from {}", path.display());
        }

        Ok(())
    }

    pub fn dry_run_check(self) -> ! {
        let mut errors = Vec::new();

        if self.p2p.clear_net.enable_inbound {
            let port = p2p_port(self.p2p.clear_net.p2p_port, self.network);
            let ip = self.p2p.clear_net.listen_on;

            match Self::check_port(IpAddr::V4(ip), port) {
                Ok(()) => println!("P2P clearnet {ip}:{port} available."),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    errors.push(e);
                }
            }
        }

        if self.p2p.clear_net.enable_inbound_v6 {
            let port = p2p_port(self.p2p.clear_net.p2p_port, self.network);
            let ip = self.p2p.clear_net.listen_on_v6;

            match Self::check_port(IpAddr::V6(ip), port) {
                Ok(()) => println!("P2P clearnet {ip}:{port} available."),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    errors.push(e);
                }
            }
        }

        if self.rpc.restricted.enable {
            let port = p2p_port(self.rpc.restricted.port, self.network);
            let ip = self.rpc.restricted.address;

            match Self::check_port(ip, port) {
                Ok(()) => println!("Rpc restricted {ip}:{port} available."),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    errors.push(e);
                }
            }
        }

        if self.rpc.unrestricted.enable {
            let port = p2p_port(self.rpc.unrestricted.port, self.network);
            let ip = self.rpc.unrestricted.address;

            match Self::check_port(ip, port) {
                Ok(()) => println!("Rpc unrestricted {ip}:{port} available."),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    errors.push(e);
                }
            }
        }

        if self.tor.mode != TorMode::Off {
            let port = self.tor.daemon.listening_addr.port();
            let ip = self.tor.daemon.listening_addr.ip();

            match Self::check_port(ip, port) {
                Ok(()) => println!("Tor daemon {ip}:{port} available."),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    errors.push(e);
                }
            }
        }

        match Self::check_dir_permissions(&self.fs.data_directory) {
            Ok(()) => println!("Permissions are ok at {}", self.fs.data_directory.display()),
            Err(e) => {
                eprintln_red(&format!("Error: {e}"));
                errors.push(e);
            }
        }

        match Self::check_dir_permissions(&self.fs.cache_directory) {
            Ok(()) => println!(
                "Permissions are ok at {}",
                self.fs.cache_directory.display()
            ),
            Err(e) => {
                eprintln_red(&format!("Error {e}"));
                errors.push(e);
            }
        }

        if self.tor.mode == TorMode::Arti {
            match Self::check_dir_permissions(&self.tor.arti.directory_path) {
                Ok(()) => println!(
                    "Permissions are ok at {}",
                    self.tor.arti.directory_path.display()
                ),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    errors.push(e);
                }
            }
        }

        let code = if errors.is_empty() {
            println!("All checks passed successfully!");
            0
        } else {
            eprintln_red("Checks failed.");
            1
        };

        std::process::exit(code)
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "========== CONFIGURATION ==========\n{self:#?}\n==================================="
        )
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use std::fs;
    use tempfile::tempdir;
    use toml::{from_str, to_string};

    use super::*;

    #[test]
    fn documented_config() {
        let str = Config::documented_config();
        let conf: Config = from_str(&str).unwrap();

        assert_eq!(conf, Config::default());
    }

    #[test]
    fn test_check_port() {
        let port = 18080;
        let ip = IpAddr::from_str("127.0.0.1").unwrap();
        assert!(Config::check_port(ip, port).is_ok());

        let _listener = TcpListener::bind((ip, port)).expect("fail to bind to the port for test");
        assert!(Config::check_port(ip, port).is_err());
    }

    #[test]
    fn test_read_from_path() {
        let tmp_dir = tempdir().unwrap();
        let config_path = tmp_dir.path().join("config.toml");
        let config_str = to_string(&Config::default()).unwrap();
        fs::write(&config_path, config_str).unwrap();

        let config = Config::read_from_path(config_path).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_check_file_permissions() {
        let tmp_dir = tempdir().unwrap();
        let path = tmp_dir.path().join("new_dir");

        // Test on non existing directory
        assert!(!path.exists());
        assert!(Config::check_dir_permissions(&path).is_ok());
        assert!(path.exists());

        // Test on an existing directory
        assert!(Config::check_dir_permissions(&path).is_ok());
    }
}
