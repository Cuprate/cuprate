//! cuprated config
use std::{
    fmt,
    fs::{read_to_string, File},
    io,
    net::{IpAddr, TcpListener},
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
    time::Duration,
};

use anyhow::bail;
use clap::Parser;
use cuprate_blockchain::config::CacheSizes;
use serde::{Deserialize, Serialize};

use cuprate_consensus::ContextConfig;
use cuprate_helper::{
    fs::{path_with_network, CUPRATE_CONFIG_DIR, DEFAULT_CONFIG_FILE_NAME},
    network::Network,
};
use cuprate_p2p::block_downloader::BlockDownloaderConfig;
use cuprate_p2p_core::{ClearNet, Tor};
use cuprate_wire::OnionAddr;

use crate::{
    logging::eprintln_red,
    tor::{TorContext, TorMode},
};

#[cfg(feature = "arti")]
use {arti_client::KeystoreSelector, safelog::DisplayRedacted};

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

pub use args::Args;
use default::DefaultOrCustom;
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

/// A lazy-lock that reads and stores total system memory.
static MEMORY: LazyLock<u64> = LazyLock::new(|| {
    tracing::info!("Attempting to read total memory from system");

    let mut info = sysinfo::System::new();
    info.refresh_memory();

    let memory = info.total_memory();

    if memory == 0 {
        eprintln_red("Unable to read total memory, please manually set the `target_max_memory` value in the config file.");
        std::process::exit(1);
    }

    memory
});

/// Finds and reads a config file from the default locations.
///
/// Tries the current directory first, then the config directory.
/// Returns `None` if no config file is found in either location.
///
/// # Errors
///
/// Returns an error if a config file is found but cannot be parsed.
pub fn find_config() -> Result<Option<Config>, anyhow::Error> {
    let paths = [
        std::env::current_dir()
            .ok()
            .map(|p| p.join(DEFAULT_CONFIG_FILE_NAME)),
        Some(CUPRATE_CONFIG_DIR.join(DEFAULT_CONFIG_FILE_NAME)),
    ];

    for path in paths.into_iter().flatten() {
        if !path.exists() {
            continue;
        }

        return Config::read_from_path(&path).map(Some);
    }

    Ok(None)
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

        /// The target maximum amount of memory to use in bytes.
        ///
        /// This is not a hard limit, but Cuprate will attempt to stay under this value.
        /// You probably do not need to change this unless Cuprate can't read the amount of RAM your
        /// system has.
        ///
        /// Type         | Number
        /// Valid values | > 0
        /// Examples     | 500_000_000, 1_000_000_000,
        pub target_max_memory: DefaultOrCustom<u64>,

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
            target_max_memory: DefaultOrCustom::Default,
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
    pub fn read_from_path(file: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let file_text = read_to_string(file.as_ref())?;

        let config: Self = toml::from_str(&file_text)?;

        tracing::info!("Using config at: {}", file.as_ref().to_string_lossy());

        Ok(config)
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
            TorMode::Daemon => inbound_enabled.then(||
                OnionAddr::new(
                    &self.tor.daemon.anonymous_inbound,
                    tor_p2p_port
                ).expect("Unable to parse supplied `anonymous_inbound` onion address. Please make sure the address is correct.")),
            #[cfg(feature = "arti")]
            TorMode::Arti => inbound_enabled.then(|| {
                let addr = ctx.arti_onion_service
                    .as_ref()
                    .unwrap()
                    .generate_identity_key(KeystoreSelector::Primary)
                    .unwrap()
                    .display_unredacted()
                    .to_string();

                OnionAddr::new(&addr, tor_p2p_port).unwrap()
            }),
            TorMode::Auto => unreachable!("Auto mode should be resolved before this point"),
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

        cuprate_blockchain::config::Config {
            blob_dir: path_with_network(&self.fs.fast_data_directory, self.network),
            index_dir: path_with_network(&self.fs.slow_data_directory, self.network),
            cache_sizes: self.storage.blockchain.tapes_cache_sizes.clone(),
        }
    }

    /// The directory for fjall.
    pub fn fjall_directory(&self) -> PathBuf {
        path_with_network(&self.fs.fast_data_directory, self.network).join("fjall")
    }

    /// Returns the size of the fjall cache.
    pub fn fjall_cache_size(&self) -> u64 {
        *self
            .storage
            .fjall_cache_size
            .value(&(self.target_max_memory() / 4))
    }

    /// Returns the target maximum memory usage.
    pub fn target_max_memory(&self) -> u64 {
        match self.target_max_memory {
            DefaultOrCustom::Default => *MEMORY,
            DefaultOrCustom::Custom(size) => size,
        }
    }

    /// The [`BlockDownloaderConfig`].
    pub fn block_downloader_config(&self) -> BlockDownloaderConfig {
        self.p2p
            .block_downloader
            .construct_inner(self.target_max_memory())
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
        let mut error = false;

        if self.p2p.clear_net.enable_inbound {
            let port = p2p_port(self.p2p.clear_net.p2p_port, self.network);
            let ip = self.p2p.clear_net.listen_on;

            match Self::check_port(IpAddr::V4(ip), port) {
                Ok(()) => println!("P2P clearnet {ip}:{port} available."),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    error = true;
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
                    error = true;
                }
            }
        }

        if self.rpc.restricted.enable {
            let port = restricted_rpc_port(self.rpc.restricted.port, self.network);
            let ip = self.rpc.restricted.address;

            match Self::check_port(ip, port) {
                Ok(()) => println!("RPC restricted {ip}:{port} available."),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    error = true;
                }
            }
        }

        if self.rpc.unrestricted.enable {
            let port = unrestricted_rpc_port(self.rpc.unrestricted.port, self.network);
            let ip = self.rpc.unrestricted.address;

            match Self::check_port(ip, port) {
                Ok(()) => println!("RPC unrestricted {ip}:{port} available."),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    error = true;
                }
            }
        }

        if self.tor.mode == TorMode::Daemon {
            let port = self.tor.daemon.listening_addr.port();
            let ip = self.tor.daemon.listening_addr.ip();

            match Self::check_port(ip, port) {
                Ok(()) => println!("Tor daemon {ip}:{port} available."),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    error = true;
                }
            }
        }

        match Self::check_dir_permissions(&self.fs.fast_data_directory) {
            Ok(()) => println!(
                "Permissions are ok at {}",
                self.fs.fast_data_directory.display()
            ),
            Err(e) => {
                eprintln_red(&format!("Error: {e}"));
                error = true;
            }
        }

        match Self::check_dir_permissions(&self.fs.slow_data_directory) {
            Ok(()) => println!(
                "Permissions are ok at {}",
                self.fs.slow_data_directory.display()
            ),
            Err(e) => {
                eprintln_red(&format!("Error: {e}"));
                error = true;
            }
        }

        match Self::check_dir_permissions(&self.fs.cache_directory) {
            Ok(()) => println!(
                "Permissions are ok at {}",
                self.fs.cache_directory.display()
            ),
            Err(e) => {
                eprintln_red(&format!("Error {e}"));
                error = true;
            }
        }

        #[cfg(feature = "arti")]
        if matches!(self.tor.mode, TorMode::Arti | TorMode::Auto) {
            match Self::check_dir_permissions(&self.tor.arti.directory_path) {
                Ok(()) => println!(
                    "Permissions are ok at {}",
                    self.tor.arti.directory_path.display()
                ),
                Err(e) => {
                    eprintln_red(&format!("Error: {e}"));
                    error = true;
                }
            }
        }

        let code = if error {
            eprintln_red("Checks failed.");
            1
        } else {
            println!("All checks passed successfully!");
            0
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
