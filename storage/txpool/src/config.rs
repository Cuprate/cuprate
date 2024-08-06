use cuprate_database::config::{Config as DbConfig, SyncMode};
use cuprate_database_service::ReaderThreads;
use cuprate_helper::fs::{cuprate_blockchain_dir, cuprate_txpool_dir};
use std::borrow::Cow;
use std::path::Path;
use cuprate_database::resize::ResizeAlgorithm;

/// The default transaction pool weight limit.
const DEFAULT_TXPOOL_WEIGHT_LIMIT: usize = 600 * 1024 * 1024;

//---------------------------------------------------------------------------------------------------- ConfigBuilder
/// Builder for [`Config`].
///
// SOMEDAY: there's are many more options to add in the future.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConfigBuilder {
    /// [`Config::db_directory`].
    db_directory: Option<Cow<'static, Path>>,

    /// [`Config::cuprate_database_config`].
    db_config: cuprate_database::config::ConfigBuilder,

    /// [`Config::reader_threads`].
    reader_threads: Option<ReaderThreads>,

    /// [`Config::max_txpool_weight`].
    max_txpool_weight: Option<usize>,
}

impl ConfigBuilder {
    /// Create a new [`ConfigBuilder`].
    ///
    /// [`ConfigBuilder::build`] can be called immediately
    /// after this function to use default values.
    pub fn new() -> Self {
        Self {
            db_directory: None,
            db_config: cuprate_database::config::ConfigBuilder::new(Cow::Borrowed(
                cuprate_blockchain_dir(),
            )),
            reader_threads: None,
            max_txpool_weight: None
        }
    }

    /// Build into a [`Config`].
    ///
    /// # Default values
    /// If [`ConfigBuilder::db_directory`] was not called,
    /// the default [`cuprate_blockchain_dir`] will be used.
    ///
    /// For all other values, [`Default::default`] is used.
    pub fn build(self) -> Config {
        // INVARIANT: all PATH safety checks are done
        // in `helper::fs`. No need to do them here.
        let db_directory = self
            .db_directory
            .unwrap_or_else(|| Cow::Borrowed(cuprate_blockchain_dir()));

        let reader_threads = self.reader_threads.unwrap_or_default();

        let max_txpool_weight = self.max_txpool_weight.unwrap_or(DEFAULT_TXPOOL_WEIGHT_LIMIT);

        let db_config = self
            .db_config
            .db_directory(db_directory)
            .reader_threads(reader_threads.as_threads())
            .build();

        Config {
            db_config,
            reader_threads,
            max_txpool_weight
        }
    }

    /// Sets a new maximum weight for the transaction pool.
    pub const fn max_txpool_weight(mut self, max_txpool_weight: usize) -> Self {
        self.max_txpool_weight = Some(max_txpool_weight);
        self
    }

    /// Set a custom database directory (and file) [`Path`].
    #[must_use]
    pub fn db_directory(mut self, db_directory: Cow<'static, Path>) -> Self {
        self.db_directory = Some(db_directory);
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
        self.db_config =
            cuprate_database::config::ConfigBuilder::new(Cow::Borrowed(cuprate_blockchain_dir()))
                .fast();

        self.reader_threads = Some(ReaderThreads::OnePerThread);
        self
    }

    /// Tune the [`ConfigBuilder`] for the lowest performing,
    /// but also least resource-intensive settings.
    ///
    /// Good default for resource-limited machines, e.g. a cheap VPS.
    #[must_use]
    pub fn low_power(mut self) -> Self {
        self.db_config =
            cuprate_database::config::ConfigBuilder::new(Cow::Borrowed(cuprate_blockchain_dir()))
                .low_power();

        self.reader_threads = Some(ReaderThreads::One);
        self
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        let db_directory = Cow::Borrowed(cuprate_blockchain_dir());
        Self {
            db_directory: Some(db_directory.clone()),
            db_config: cuprate_database::config::ConfigBuilder::new(db_directory),
            reader_threads: Some(ReaderThreads::default()),
            max_txpool_weight: Some(DEFAULT_TXPOOL_WEIGHT_LIMIT)
        }
    }
}

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
