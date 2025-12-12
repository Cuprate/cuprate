//! Database configuration.
//!
//! This module contains the main [`Config`]uration struct
//! for the database [`Env`](cuprate_database::Env)ironment,
//! and blockchain-specific configuration.
//!
//! It also contains types related to configuration settings.
//!
//! The main constructor is the [`ConfigBuilder`].
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
use tapes::{Advice, MmapFile, MmapFileOpenOption, Tape};

// re-exports
pub use cuprate_database_service::ReaderThreads;

use crate::database::{
    BLOCK_INFOS, PRUNABLE_BLOBS, PRUNED_BLOBS, RCT_OUTPUTS, TX_INFOS, V1_PRUNABLE_BLOBS,
};

//---------------------------------------------------------------------------------------------------- ConfigBuilder
/// Builder for [`Config`].
///
// SOMEDAY: there's are many more options to add in the future.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConfigBuilder {
    network: Network,

    data_dir: Option<PathBuf>,

    /// [`Config::reader_threads`].
    reader_threads: Option<ReaderThreads>,
}

impl ConfigBuilder {
    /// Create a new [`ConfigBuilder`].
    ///
    /// [`ConfigBuilder::build`] can be called immediately
    /// after this function to use default values.
    pub fn new() -> Self {
        Self {
            network: Network::default(),
            data_dir: None,
            reader_threads: None,
        }
    }

    /// Build into a [`Config`].
    ///
    /// # Default values
    /// If [`ConfigBuilder::data_directory`] was not called,
    /// [`blockchain_path`] with [`CUPRATE_DATA_DIR`] [`Network::Mainnet`] will be used.
    ///
    /// For all other values, [`Default::default`] is used.
    pub fn build(self) -> Config {
        // INVARIANT: all PATH safety checks are done
        // in `helper::fs`. No need to do them here.
        let data_dir = self
            .data_dir
            .unwrap_or_else(|| CUPRATE_DATA_DIR.to_path_buf());

        let reader_threads = self.reader_threads.unwrap_or_default();

        Config {
            data_dir: blockchain_path(&data_dir, self.network),
            blob_data_dir: None,
            reader_threads,
        }
    }

    /// Change the network this blockchain database is for.
    #[must_use]
    pub const fn network(mut self, network: Network) -> Self {
        self.network = network;
        self
    }

    /// Set a custom database directory (and file) [`PathBuf`].
    #[must_use]
    pub fn data_directory(mut self, db_directory: PathBuf) -> Self {
        self.data_dir = Some(db_directory);
        self
    }

    /// Set a custom [`ReaderThreads`].
    #[must_use]
    pub const fn reader_threads(mut self, reader_threads: ReaderThreads) -> Self {
        self.reader_threads = Some(reader_threads);
        self
    }

    /// Tune the [`ConfigBuilder`] for the highest performing,
    /// but also most resource-intensive & maybe risky settings.
    ///
    /// Good default for testing, and resource-available machines.
    #[must_use]
    pub fn fast(mut self) -> Self {
        self.reader_threads = Some(ReaderThreads::OnePerThread);
        self
    }

    /// Tune the [`ConfigBuilder`] for the lowest performing,
    /// but also least resource-intensive settings.
    ///
    /// Good default for resource-limited machines, e.g. a cheap VPS.
    #[must_use]
    pub fn low_power(mut self) -> Self {
        self.reader_threads = Some(ReaderThreads::One);
        self
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            network: Network::default(),
            data_dir: Some(blockchain_path(&CUPRATE_DATA_DIR, Network::default())),
            reader_threads: Some(ReaderThreads::default()),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Config
/// `cuprate_blockchain` configuration.
///
/// This is a configuration built on-top of [`cuprate_database::config::Config`].
///
/// It contains configuration specific to this crate, plus the database config.
///
/// For construction, either use [`ConfigBuilder`] or [`Config::default`].
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    pub data_dir: PathBuf,

    /// The directory to store block/tx blobs.
    pub blob_data_dir: Option<PathBuf>,

    /// Database reader thread count.
    pub reader_threads: ReaderThreads,
}

impl Config {
    /// Create a new [`Config`] with sane default settings.
    ///
    /// The [`cuprate_database::config::Config::db_directory`]
    /// will be set to [`blockchain_path`] with [`CUPRATE_DATA_DIR`] [`Network::Mainnet`].
    ///
    /// All other values will be [`Default::default`].
    ///
    /// Same as [`Config::default`].
    ///
    /// ```rust
    /// use cuprate_database::{
    ///     config::SyncMode,
    ///     resize::ResizeAlgorithm,
    ///     DATABASE_DATA_FILENAME,
    /// };
    /// use cuprate_helper::{fs::*, network::Network};
    ///
    /// use cuprate_blockchain::config::*;
    ///
    /// let config = Config::new();
    ///
    /// assert_eq!(config.db_config.db_directory().as_ref(), blockchain_path(&CUPRATE_DATA_DIR, Network::Mainnet).as_path());
    /// assert!(config.db_config.db_file().starts_with(&*CUPRATE_DATA_DIR));
    /// assert!(config.db_config.db_file().ends_with(DATABASE_DATA_FILENAME));
    /// assert_eq!(config.db_config.sync_mode, SyncMode::default());
    /// assert_eq!(config.db_config.resize_algorithm, ResizeAlgorithm::default());
    /// assert_eq!(config.reader_threads, ReaderThreads::default());
    /// ```
    pub fn new() -> Self {
        ConfigBuilder::default().build()
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

pub fn linear_tapes_config(
    data_dir: PathBuf,
    blob_data_dir: Option<PathBuf>,
) -> Vec<Tape<MmapFile>> {
    [
        Tape {
            name: RCT_OUTPUTS,
            advice: Advice::Random,
            backing_memory_options: MmapFileOpenOption {
                dir: data_dir.clone(),
            },
            initial_memory_size: 0,
        },
        Tape {
            name: TX_INFOS,
            backing_memory_options: MmapFileOpenOption {
                dir: data_dir.clone(),
            },
            initial_memory_size: 0,
            advice: Advice::Random,
        },
        Tape {
            name: BLOCK_INFOS,
            backing_memory_options: MmapFileOpenOption {
                dir: data_dir.clone(),
            },
            initial_memory_size: 0,
            advice: Advice::Random,
        },
        Tape {
            name: PRUNED_BLOBS,
            backing_memory_options: MmapFileOpenOption {
                dir: blob_data_dir.clone().unwrap_or_else(|| data_dir.clone()),
            },
            initial_memory_size: 0,
            advice: Advice::Sequential,
        },
        Tape {
            name: V1_PRUNABLE_BLOBS,
            backing_memory_options: MmapFileOpenOption {
                dir: blob_data_dir.clone().unwrap_or_else(|| data_dir.clone()),
            },
            initial_memory_size: 0,
            advice: Advice::Sequential,
        },
    ]
    .into_iter()
    .chain(PRUNABLE_BLOBS.into_iter().map(|name| Tape {
        name,
        backing_memory_options: MmapFileOpenOption {
            dir: blob_data_dir.clone().unwrap_or_else(|| data_dir.clone()),
        },
        initial_memory_size: 0,
        advice: Advice::Sequential,
    }))
    .collect()
}
