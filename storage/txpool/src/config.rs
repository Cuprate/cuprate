use cuprate_database::config::Config as DbConfig;
use cuprate_database_service::ReaderThreads;
use cuprate_helper::fs::cuprate_txpool_dir;
use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- Config
/// `cuprate_txpool` configuration.
///
/// This is a configuration built on-top of [`DbConfig`].
///
/// It contains configuration specific to this crate, plus the database config.
///
/// For construction, either use [`ConfigBuilder`] or [`Config::default`].
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Config {
    /// The database configuration.
    pub db_config: DbConfig,

    /// Database reader thread count.
    pub reader_threads: ReaderThreads,

    /// The maximum weight of the transaction pool, after which we will start dropping transactions.
    pub max_txpool_weight: usize,
}

impl Config {
    /// Create a new [`Config`] with sane default settings.
    ///
    /// The [`DbConfig::db_directory`]
    /// will be set to [`cuprate_blockchain_dir`].
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
    /// use cuprate_database_service::ReaderThreads;
    /// use cuprate_helper::fs::*;
    ///
    /// use cuprate_txpool::Config;
    ///
    /// let config = Config::new();
    ///
    /// assert_eq!(config.db_config.db_directory(), cuprate_txpool_dir());
    /// assert!(config.db_config.db_file().starts_with(cuprate_txpool_dir()));
    /// assert!(config.db_config.db_file().ends_with(DATABASE_DATA_FILENAME));
    /// assert_eq!(config.db_config.sync_mode, SyncMode::default());
    /// assert_eq!(config.db_config.resize_algorithm, ResizeAlgorithm::default());
    /// assert_eq!(config.reader_threads, ReaderThreads::default());
    /// ```
    pub fn new() -> Self {
        Config {
            db_config: DbConfig::new(Cow::Borrowed(cuprate_txpool_dir())),
            reader_threads: ReaderThreads::default(),
            max_txpool_weight: 0,
        }
    }
}

impl Default for Config {
    /// Same as [`Config::new`].
    ///
    /// ```rust
    /// # use cuprate_txpool::Config;
    /// assert_eq!(Config::default(), Config::new());
    /// ```
    fn default() -> Self {
        Self::new()
    }
}
