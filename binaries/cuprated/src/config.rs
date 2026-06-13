//! cuprated config
use std::{
    fmt,
    fs::{read_to_string, File},
    io,
    net::{IpAddr, TcpListener},
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use anyhow::{bail, Context};
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
    logging::{eprintln_red, eprintln_yellow},
    tor::{TorContext, TorMode},
};

#[cfg(feature = "arti")]
use {arti_client::KeystoreSelector, safelog::DisplayRedacted};

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

use default::DefaultOrCustom;
use fs::FileSystemConfig;
pub use p2p::{p2p_port, P2PConfig};
use rayon::RayonConfig;
pub use rpc::{restricted_rpc_port, unrestricted_rpc_port, RpcConfig};
pub use storage::{StorageConfig, TxpoolConfig};
use tokio::TokioConfig;
use tor::TorConfig;
use tracing_config::TracingConfig;

/// Result of a single check from [`Config::dry_run_check`].
pub struct DryRunResult {
    /// Description of the check.
    pub description: String,
    /// The result of the check.
    pub result: Result<(), anyhow::Error>,
}

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

/// Resolves `target_max_memory` from system RAM if unset.
pub fn resolve_max_memory(config: &mut Config) {
    // TODO: don't use `DefaultOrCustom` for target_max_memory.
    if matches!(config.target_max_memory, DefaultOrCustom::Default) {
        tracing::info!("Attempting to read total memory from system");

        let mut info = sysinfo::System::new();
        info.refresh_memory();
        let memory = info.total_memory();

        if memory == 0 {
            eprintln_red("Unable to read total memory, please manually set the `target_max_memory` value in the config file.");
            std::process::exit(1);
        }

        config.target_max_memory = DefaultOrCustom::Custom(memory);
    }
}

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

/// Deserializes a string into `T` via its [`FromStr`] implementation.
///
/// Used with `#[serde(deserialize_with = "...")]` on config fields whose [`FromStr`]
/// implementations match strings case-insensitively, see:
/// <https://github.com/Cuprate/cuprate/issues/598>.
pub(crate) fn deserialize_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(serde::de::Error::custom)
}

/// Trims leading/trailing whitespace from every string value in the given config text,
/// printing a warning for each trimmed value.
///
/// Returns the text unchanged if it is not parseable TOML, the regular
/// parsing path will then report the error.
fn trim_config_text(text: String) -> String {
    trim_string_values(&text).map_or(text, |(trimmed_text, trimmed_paths)| {
        for path in trimmed_paths {
            eprintln_yellow(&format!(
                "Warning: ignoring leading/trailing whitespace in config value `{path}`"
            ));
        }

        trimmed_text
    })
}

/// Trims leading/trailing whitespace from every string value in the given TOML text.
///
/// Values that would become empty when trimmed are left untouched.
///
/// Returns the cleaned TOML text and the dotted key paths of the values that were
/// trimmed, or [`None`] if the text is not parseable TOML.
fn trim_string_values(text: &str) -> Option<(String, Vec<String>)> {
    let mut doc = toml_edit::DocumentMut::from_str(text).ok()?;

    let mut trimmed = Vec::new();
    trim_table(doc.as_table_mut(), "", &mut trimmed);

    Some((doc.to_string(), trimmed))
}

/// [`trim_string_values`] on every item in a table.
fn trim_table(table: &mut dyn toml_edit::TableLike, path: &str, trimmed: &mut Vec<String>) {
    for (key, item) in table.iter_mut() {
        let item_path = if path.is_empty() {
            key.get().to_string()
        } else {
            format!("{path}.{}", key.get())
        };

        trim_item(item, &item_path, trimmed);
    }
}

/// [`trim_string_values`] on a single item.
fn trim_item(item: &mut toml_edit::Item, path: &str, trimmed: &mut Vec<String>) {
    match item {
        toml_edit::Item::None => (),
        toml_edit::Item::Value(value) => trim_value(value, path, trimmed),
        toml_edit::Item::Table(table) => trim_table(table, path, trimmed),
        toml_edit::Item::ArrayOfTables(tables) => {
            for (i, table) in tables.iter_mut().enumerate() {
                trim_table(table, &format!("{path}[{i}]"), trimmed);
            }
        }
    }
}

/// [`trim_string_values`] on a single value.
fn trim_value(value: &mut toml_edit::Value, path: &str, trimmed: &mut Vec<String>) {
    match value {
        toml_edit::Value::String(s) => {
            let trimmed_string = s.value().trim().to_string();

            // Never trim a value down to the empty string, e.g. an all-whitespace
            // `proxy` must not silently become "" (proxy disabled), it is left
            // untouched for the field's own parser to reject loudly.
            if !trimmed_string.is_empty() && trimmed_string != *s.value() {
                *value = toml_edit::Value::String(toml_edit::Formatted::new(trimmed_string));
                trimmed.push(path.to_string());
            }
        }
        toml_edit::Value::Array(array) => {
            for (i, element) in array.iter_mut().enumerate() {
                trim_value(element, &format!("{path}[{i}]"), trimmed);
            }
        }
        toml_edit::Value::InlineTable(table) => trim_table(table, path, trimmed),
        toml_edit::Value::Integer(_)
        | toml_edit::Value::Float(_)
        | toml_edit::Value::Boolean(_)
        | toml_edit::Value::Datetime(_) => (),
    }
}

config_struct! {
    /// The config for all of Cuprate.
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct Config {
        /// The network cuprated should run on.
        ///
        /// This value is matched case-insensitively.
        ///
        /// Valid values | "Mainnet", "Testnet", "Stagenet", "FakeChain"
        ##[serde(deserialize_with = "deserialize_from_str")]
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

        #[comment_out = true]
        /// Fixes the PoW difficulty to this value.
        ///
        /// Only intended for regtest (`network = "FakeChain"`). A value of
        /// `0` disables this override.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        pub fixed_difficulty: u128,

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
            fixed_difficulty: 0,
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
        let file_text = trim_config_text(read_to_string(file.as_ref())?);

        let config: Self = toml::from_str(&file_text).with_context(|| {
            format!(
                "Failed to parse config file at: {}",
                file.as_ref().to_string_lossy()
            )
        })?;

        println!("Using config at: {}", file.as_ref().to_string_lossy());

        Ok(config)
    }

    /// Returns the current [`Network`] we are running on.
    pub const fn network(&self) -> Network {
        self.network
    }

    /// Returns the fast-sync validation hashes for this config's network,
    /// or `&[]` if fast sync is disabled.
    pub fn fast_sync_hashes(&self) -> &'static [[u8; 32]] {
        crate::blockchain::get_fast_sync_hashes(self.fast_sync, self.network)
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
        let mut cfg = match self.network {
            Network::Mainnet => ContextConfig::main_net(),
            Network::Stagenet => ContextConfig::stage_net(),
            Network::Testnet => ContextConfig::test_net(),
            Network::FakeChain => ContextConfig::fake_chain(),
        };

        if self.fixed_difficulty != 0 {
            cfg.difficulty_cfg.fixed_difficulty = Some(self.fixed_difficulty);
        }

        cfg
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
    ///
    /// # Panics
    ///
    /// Panics if `target_max_memory` is unresolved.
    pub fn fjall_cache_size(&self) -> u64 {
        *self
            .storage
            .fjall_cache_size
            .value(&(self.target_max_memory() / 4))
    }

    /// Returns the target maximum memory usage.
    ///
    /// # Panics
    ///
    /// Panics if `target_max_memory` is unresolved.
    pub fn target_max_memory(&self) -> u64 {
        match self.target_max_memory {
            DefaultOrCustom::Default => {
                panic!("`target_max_memory` is unresolved; call `resolve_max_memory` first")
            }
            DefaultOrCustom::Custom(size) => size,
        }
    }

    /// The [`BlockDownloaderConfig`].
    ///
    /// # Panics
    ///
    /// Panics if `target_max_memory` is unresolved.
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

    pub fn dry_run_check(&self) -> Vec<DryRunResult> {
        let mut results = Vec::new();

        if self.p2p.clear_net.enable_inbound {
            let port = p2p_port(self.p2p.clear_net.p2p_port, self.network);
            let ip = self.p2p.clear_net.listen_on;

            results.push(DryRunResult {
                description: format!("P2P clearnet {ip}:{port} available."),
                result: Self::check_port(IpAddr::V4(ip), port),
            });
        }

        if self.p2p.clear_net.enable_inbound_v6 {
            let port = p2p_port(self.p2p.clear_net.p2p_port, self.network);
            let ip = self.p2p.clear_net.listen_on_v6;

            results.push(DryRunResult {
                description: format!("P2P clearnet {ip}:{port} available."),
                result: Self::check_port(IpAddr::V6(ip), port),
            });
        }

        if self.rpc.restricted.enable {
            let port = restricted_rpc_port(self.rpc.restricted.port, self.network);
            let ip = self.rpc.restricted.address;

            results.push(DryRunResult {
                description: format!("RPC restricted {ip}:{port} available."),
                result: Self::check_port(ip, port),
            });
        }

        if self.rpc.unrestricted.enable {
            let port = unrestricted_rpc_port(self.rpc.unrestricted.port, self.network);
            let ip = self.rpc.unrestricted.address;

            results.push(DryRunResult {
                description: format!("RPC unrestricted {ip}:{port} available."),
                result: Self::check_port(ip, port),
            });
        }

        if self.tor.mode == TorMode::Daemon {
            let port = self.tor.daemon.listening_addr.port();
            let ip = self.tor.daemon.listening_addr.ip();

            results.push(DryRunResult {
                description: format!("Tor daemon {ip}:{port} available."),
                result: Self::check_port(ip, port),
            });
        }

        results.push(DryRunResult {
            description: format!(
                "File permissions are valid at {}",
                self.fs.fast_data_directory.display()
            ),
            result: Self::check_dir_permissions(&self.fs.fast_data_directory),
        });

        results.push(DryRunResult {
            description: format!(
                "File permissions are valid at {}",
                self.fs.slow_data_directory.display()
            ),
            result: Self::check_dir_permissions(&self.fs.slow_data_directory),
        });

        results.push(DryRunResult {
            description: format!(
                "File permissions are valid at {}",
                self.fs.cache_directory.display()
            ),
            result: Self::check_dir_permissions(&self.fs.cache_directory),
        });

        #[cfg(feature = "arti")]
        if matches!(self.tor.mode, TorMode::Arti | TorMode::Auto) {
            results.push(DryRunResult {
                description: format!(
                    "File permissions are valid at {}",
                    self.tor.arti.directory_path.display()
                ),
                result: Self::check_dir_permissions(&self.tor.arti.directory_path),
            });
        }

        results
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
    use crate::p2p::ProxySettings;

    #[test]
    fn documented_config() {
        let str = Config::documented_config();
        let conf: Config = from_str(&str).unwrap();

        assert_eq!(conf, Config::default());
    }

    #[test]
    fn trim_string_values_walks_nested_structure() {
        let toml = r#"
network = "  Mainnet"

[tor]
mode = "Daemon "

[p2p.clear_net]
proxy = " socks5://user:pass@127.0.0.1:9050"
"#;

        let (cleaned, trimmed) = trim_string_values(toml).unwrap();
        assert_eq!(trimmed, ["network", "tor.mode", "p2p.clear_net.proxy"]);

        let config: Config = from_str(&cleaned).unwrap();
        assert_eq!(config.network, Network::Mainnet);
        assert_eq!(config.tor.mode, TorMode::Daemon);
        assert!(matches!(
            config.p2p.clear_net.proxy,
            ProxySettings::Socks(_)
        ));
    }

    #[test]
    fn trim_string_values_arrays_and_inline_tables() {
        #[derive(Deserialize)]
        struct T {
            a: Vec<String>,
            b: B,
            d: Vec<D>,
        }
        #[derive(Deserialize)]
        struct B {
            c: String,
        }
        #[derive(Deserialize)]
        struct D {
            e: String,
        }

        let toml = r#"
a = ["  x", "y  ", "z"]
b = { c = " v " }

[[d]]
e = " w"
"#;

        let (cleaned, trimmed) = trim_string_values(toml).unwrap();
        assert_eq!(trimmed, ["a[0]", "a[1]", "b.c", "d[0].e"]);

        let t: T = from_str(&cleaned).unwrap();
        assert_eq!(t.a, ["x", "y", "z"]);
        assert_eq!(t.b.c, "v");
        assert_eq!(t.d[0].e, "w");
    }

    #[test]
    fn trim_string_values_no_changes() {
        let toml = "a = \"x\"\nb = 1\n";

        let (cleaned, trimmed) = trim_string_values(toml).unwrap();
        assert!(trimmed.is_empty());
        assert_eq!(cleaned, toml);
    }

    #[test]
    fn trim_string_values_keeps_whitespace_only_values() {
        // An all-whitespace proxy must NOT be trimmed to "" (proxy disabled),
        // the field's own parser must reject it like it did before trimming existed.
        let toml = "[p2p.clear_net]\nproxy = \" \"\n";

        let (cleaned, trimmed) = trim_string_values(toml).unwrap();
        assert!(trimmed.is_empty());
        assert_eq!(cleaned, toml);
        assert!(from_str::<Config>(&cleaned).is_err());
    }

    #[test]
    fn trim_string_values_invalid_toml() {
        assert!(trim_string_values("a = = 1").is_none());
    }

    #[test]
    fn config_values_ignore_ascii_case() {
        let config: Config = from_str(r#"network = "MAINNET""#).unwrap();
        assert_eq!(config.network, Network::Mainnet);

        let config: Config = from_str(r#"network = "fakechain""#).unwrap();
        assert_eq!(config.network, Network::FakeChain);

        let config: Config = from_str("[tor]\nmode = \"daemon\"").unwrap();
        assert_eq!(config.tor.mode, TorMode::Daemon);

        let config: Config = from_str(r#"target_max_memory = "default""#).unwrap();
        assert_eq!(config.target_max_memory, DefaultOrCustom::Default);

        let config: Config = from_str("[p2p.clear_net]\nproxy = \"tor\"").unwrap();
        assert_eq!(config.p2p.clear_net.proxy, ProxySettings::Tor);

        let config: Config = from_str("[p2p.clear_net]\nproxy = \"TOR\"").unwrap();
        assert_eq!(config.p2p.clear_net.proxy, ProxySettings::Tor);

        let config: Config =
            from_str("[p2p.clear_net]\nproxy = \"SOCKS5://127.0.0.1:9050\"").unwrap();
        assert!(matches!(
            config.p2p.clear_net.proxy,
            ProxySettings::Socks(_)
        ));
    }

    #[test]
    fn config_values_still_reject_invalid_strings() {
        assert!(from_str::<Config>(r#"network = "mainnet2""#).is_err());
        assert!(from_str::<Config>("network = 5").is_err());
        assert!(from_str::<Config>("[tor]\nmode = \"daemonn\"").is_err());
        assert!(from_str::<Config>(r#"target_max_memory = "defaults""#).is_err());
        assert!(from_str::<Config>("target_max_memory = -5").is_err());
    }

    #[test]
    fn default_or_custom_custom_values_parse() {
        let config: Config = from_str("target_max_memory = 1000").unwrap();
        assert_eq!(config.target_max_memory, DefaultOrCustom::Custom(1000));
    }

    #[test]
    fn read_from_path_trims_string_values() {
        let tmp_dir = tempdir().unwrap();
        let config_path = tmp_dir.path().join("config.toml");
        fs::write(&config_path, "network = \" Mainnet \"\n").unwrap();

        let config = Config::read_from_path(config_path).unwrap();
        assert_eq!(config.network(), Network::Mainnet);
    }

    #[test]
    fn read_from_path_invalid_toml_still_errors() {
        let tmp_dir = tempdir().unwrap();
        let config_path = tmp_dir.path().join("config.toml");
        fs::write(&config_path, "network = = 1\n").unwrap();

        assert!(Config::read_from_path(config_path).is_err());
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
