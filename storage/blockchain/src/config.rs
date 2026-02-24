//! Database configuration.
//!
//! It also contains types related to configuration settings.
//!
//! These configurations are processed at runtime, meaning
//! the `Env` can/will dynamically adjust its behavior based
//! on these values.
//!
//! # Example
//! ```rust
//! use cuprate_blockchain::{
//!     cuprate_database::{Env, config::SyncMode},
//!     config::{ConfigBuilder, ReaderThreads},
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let tmp_dir = tempfile::tempdir()?;
//! let db_dir = tmp_dir.path().to_owned();
//!
//! let config = ConfigBuilder::new()
//!      // Use a custom database directory.
//!     .data_directory(db_dir.into())
//!     // Use as many reader threads as possible (when using `service`).
//!     .reader_threads(ReaderThreads::OnePerThread)
//!     // Use the fastest sync mode.
//!     .sync_mode(SyncMode::Fast)
//!     // Build into `Config`
//!     .build();
//!
//! // Start a database `service` using this configuration.
//! let (_, _, env) = cuprate_blockchain::service::init(config.clone())?;
//! // It's using the config we provided.
//! assert_eq!(env.config(), &config.db_config);
//! # Ok(()) }
//! ```

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, path::PathBuf};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_helper::{
    fs::{blockchain_path, CUPRATE_DATA_DIR},
    network::Network,
};

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields, default))]
pub struct CacheSizes {
    pub rct_outputs: u64,
    pub tx_infos: u64,
    pub block_infos: u64,
    pub pruned_blobs: u64,
    pub v1_prunable_blobs: u64,
    pub prunable_blobs: u64,
}

impl Default for CacheSizes {
    fn default() -> Self {
        Self {
            rct_outputs: 100 * 1024 * 1024,
            tx_infos: 1024 * 1024,
            block_infos: 1024 * 1024,
            pruned_blobs: 25 * 1024 * 1024,
            v1_prunable_blobs: 8 * 1024,
            prunable_blobs: 8 * 1024,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Config
/// TODO.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    pub blob_dir: PathBuf,
    pub index_dir: PathBuf,

    pub cache_sizes: CacheSizes,
}

impl Config {
    /// TODO
    pub fn new() -> Self {
        Self {
            blob_dir: blockchain_path(&CUPRATE_DATA_DIR, Network::Mainnet),
            index_dir: blockchain_path(&CUPRATE_DATA_DIR, Network::Mainnet),
            cache_sizes: CacheSizes::default(),
        }
    }
}

impl Default for Config {
    /// Same as [`Config::new`].
    ///
    /// ```rust
    /// # use cuprate_blockchain::config::*;
    /// assert_eq!(Config::default(), Config::new());
    /// ```
    fn default() -> Self {
        Self::new()
    }
}
