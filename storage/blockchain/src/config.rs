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
use std::num::NonZeroUsize;
use std::sync::Arc;
use rayon::ThreadPool;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_database::{config::SyncMode, resize::ResizeAlgorithm};
use cuprate_helper::{
    fs::{blockchain_path, CUPRATE_DATA_DIR},
    network::Network,
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

    /// [`Config::cuprate_database_config`].
    db_config: cuprate_database::config::ConfigBuilder,

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
            db_config: cuprate_database::config::ConfigBuilder::new(Cow::Owned(blockchain_path(
                &CUPRATE_DATA_DIR,
                Network::Mainnet,
            ))),
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
        let db_config = self
            .db_config
            .db_directory(Cow::Owned(blockchain_path(&data_dir, self.network)))
            .reader_threads(reader_threads.as_threads())
            .build();

        Config {
            db_config,
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

    /// Calls [`cuprate_database::config::ConfigBuilder::sync_mode`].
    #[must_use]
    pub fn sync_mode(mut self, sync_mode: SyncMode) -> Self {
        self.db_config = self.db_config.sync_mode(sync_mode);
        self
    }

    /// Calls [`cuprate_database::config::ConfigBuilder::resize_algorithm`].
    #[must_use]
    pub fn resize_algorithm(mut self, resize_algorithm: ResizeAlgorithm) -> Self {
        self.db_config = self.db_config.resize_algorithm(resize_algorithm);
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
        self.db_config = self.db_config.fast();

        self.reader_threads = Some(ReaderThreads::OnePerThread);
        self
    }

    /// Tune the [`ConfigBuilder`] for the lowest performing,
    /// but also least resource-intensive settings.
    ///
    /// Good default for resource-limited machines, e.g. a cheap VPS.
    #[must_use]
    pub fn low_power(mut self) -> Self {
        self.db_config = self.db_config.low_power();

        self.reader_threads = Some(ReaderThreads::One);
        self
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            network: Network::default(),
            data_dir: Some(CUPRATE_DATA_DIR.to_path_buf()),
            db_config: cuprate_database::config::ConfigBuilder::new(Cow::Owned(blockchain_path(
                &CUPRATE_DATA_DIR,
                Network::default(),
            ))),
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
    /// The database configuration.
    pub db_config: cuprate_database::config::Config,

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


//---------------------------------------------------------------------------------------------------- init_thread_pool
/// Initialize the reader thread-pool backed by `rayon`.
pub fn init_thread_pool(reader_threads: ReaderThreads) -> Arc<ThreadPool> {
    // How many reader threads to spawn?
    let reader_count = reader_threads.as_threads().get();

    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(reader_count)
            .thread_name(|i| format!("{}::DatabaseReader({i})", module_path!()))
            .build()
            .unwrap(),
    )
}

//---------------------------------------------------------------------------------------------------- ReaderThreads
/// Amount of database reader threads to spawn.
///
/// This controls how many reader threads the [`DatabaseReadService`](crate::DatabaseReadService)
/// thread-pool will spawn to receive and send requests/responses.
///
/// # Invariant
/// The main function used to extract an actual
/// usable thread count out of this is [`ReaderThreads::as_threads`].
///
/// This will always return at least 1, up until the amount of threads on the machine.
#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ReaderThreads {
    #[default]
    /// Spawn 1 reader thread per available thread on the machine.
    ///
    /// For example, a `32-thread` system will spawn
    /// `32` reader threads using this setting.
    OnePerThread,

    /// Only spawn 1 reader thread.
    One,

    /// Spawn a specified amount of reader threads.
    ///
    /// Note that no matter how large this value, it will be
    /// ultimately capped at the amount of system threads.
    ///
    /// # `0`
    /// `ReaderThreads::Number(0)` represents "use maximum value",
    /// as such, it is equal to [`ReaderThreads::OnePerThread`].
    ///
    /// ```rust
    /// # use cuprate_database_service::*;
    /// let reader_threads = ReaderThreads::from(0_usize);
    /// assert!(matches!(reader_threads, ReaderThreads::OnePerThread));
    /// ```
    Number(usize),

    /// Spawn a specified % of reader threads.
    ///
    /// This must be a value in-between `0.0..1.0`
    /// where `1.0` represents [`ReaderThreads::OnePerThread`].
    ///
    /// # Example
    /// For example, using a `16-core, 32-thread` Ryzen 5950x CPU:
    ///
    /// | Input                              | Total thread used |
    /// |------------------------------------|-------------------|
    /// | `ReaderThreads::Percent(0.0)`      | 32 (maximum value)
    /// | `ReaderThreads::Percent(0.5)`      | 16
    /// | `ReaderThreads::Percent(0.75)`     | 24
    /// | `ReaderThreads::Percent(1.0)`      | 32
    /// | `ReaderThreads::Percent(2.0)`      | 32 (saturating)
    /// | `ReaderThreads::Percent(f32::NAN)` | 32 (non-normal default)
    ///
    /// # `0.0`
    /// `ReaderThreads::Percent(0.0)` represents "use maximum value",
    /// as such, it is equal to [`ReaderThreads::OnePerThread`].
    ///
    /// # Not quite `0.0`
    /// If the thread count multiplied by the percentage ends up being
    /// non-zero, but not 1 thread, the minimum value 1 will be returned.
    ///
    /// ```rust
    /// # use cuprate_database_service::ReaderThreads;
    /// assert_eq!(ReaderThreads::Percent(0.000000001).as_threads().get(), 1);
    /// ```
    Percent(f32),
}

impl ReaderThreads {
    /// This converts [`ReaderThreads`] into a safe, usable
    /// number representing how many threads to spawn.
    ///
    /// This function will always return a number in-between `1..=total_thread_count`.
    ///
    /// It uses [`cuprate_helper::thread::threads()`] internally to determine the total thread count.
    ///
    /// # Example
    /// ```rust
    /// use cuprate_database_service::ReaderThreads as R;
    ///
    /// let total_threads: std::num::NonZeroUsize =
    ///     cuprate_helper::thread::threads();
    ///
    /// assert_eq!(R::OnePerThread.as_threads(), total_threads);
    ///
    /// assert_eq!(R::One.as_threads().get(), 1);
    ///
    /// assert_eq!(R::Number(0).as_threads(), total_threads);
    /// assert_eq!(R::Number(1).as_threads().get(), 1);
    /// assert_eq!(R::Number(usize::MAX).as_threads(), total_threads);
    ///
    /// assert_eq!(R::Percent(0.01).as_threads().get(), 1);
    /// assert_eq!(R::Percent(0.0).as_threads(), total_threads);
    /// assert_eq!(R::Percent(1.0).as_threads(), total_threads);
    /// assert_eq!(R::Percent(f32::NAN).as_threads(), total_threads);
    /// assert_eq!(R::Percent(f32::INFINITY).as_threads(), total_threads);
    /// assert_eq!(R::Percent(f32::NEG_INFINITY).as_threads(), total_threads);
    ///
    /// // Percentage only works on more than 1 thread.
    /// if total_threads.get() > 1 {
    ///     assert_eq!(
    ///         R::Percent(0.5).as_threads().get(),
    ///         (total_threads.get() as f32 / 2.0) as usize,
    ///     );
    /// }
    /// ```
    //
    // INVARIANT:
    // LMDB will error if we input zero, so don't allow that.
    // <https://github.com/LMDB/lmdb/blob/b8e54b4c31378932b69f1298972de54a565185b1/libraries/liblmdb/mdb.c#L4687>
    pub fn as_threads(&self) -> NonZeroUsize {
        let total_threads = cuprate_helper::thread::threads();

        match self {
            Self::OnePerThread => total_threads, // use all threads
            Self::One => NonZeroUsize::MIN,      // one
            Self::Number(n) => match NonZeroUsize::new(*n) {
                Some(n) => std::cmp::min(n, total_threads), // saturate at total threads
                None => total_threads,                      // 0 == maximum value
            },

            // We handle the casting loss.
            #[expect(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            Self::Percent(f) => {
                // If non-normal float, use the default (all threads).
                if !f.is_normal() || !(0.0..=1.0).contains(f) {
                    return total_threads;
                }

                // 0.0 == maximum value.
                if *f == 0.0 {
                    return total_threads;
                }

                // Calculate percentage of total threads.
                let thread_percent = (total_threads.get() as f32) * f;
                match NonZeroUsize::new(thread_percent as usize) {
                    Some(n) => std::cmp::min(n, total_threads), // saturate at total threads.
                    None => {
                        // We checked for `0.0` above, so what this
                        // being 0 means that the percentage was _so_
                        // low it made our thread count something like
                        // 0.99. In this case, just use 1 thread.
                        NonZeroUsize::MIN
                    }
                }
            }
        }
    }
}

impl<T: Into<usize>> From<T> for ReaderThreads {
    /// Create a [`ReaderThreads::Number`].
    ///
    /// If `value` is `0`, this will return [`ReaderThreads::OnePerThread`].
    fn from(value: T) -> Self {
        let u: usize = value.into();
        if u == 0 {
            Self::OnePerThread
        } else {
            Self::Number(u)
        }
    }
}

