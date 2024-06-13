//! The main [`Config`] struct, holding all configurable values.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_helper::fs::cuprate_blockchain_dir;

use crate::{
    config::{ReaderThreads, SyncMode},
    constants::DATABASE_DATA_FILENAME,
    resize::ResizeAlgorithm,
};

//---------------------------------------------------------------------------------------------------- ConfigBuilder
/// Builder for [`Config`].
///
// SOMEDAY: there's are many more options to add in the future.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConfigBuilder {
    /// [`Config::db_directory`].
    db_directory: Option<Cow<'static, Path>>,

    /// [`Config::sync_mode`].
    sync_mode: Option<SyncMode>,

    /// [`Config::reader_threads`].
    reader_threads: Option<ReaderThreads>,

    /// [`Config::resize_algorithm`].
    resize_algorithm: Option<ResizeAlgorithm>,
}

impl ConfigBuilder {
    /// Create a new [`ConfigBuilder`].
    ///
    /// [`ConfigBuilder::build`] can be called immediately
    /// after this function to use default values.
    pub const fn new() -> Self {
        Self {
            db_directory: None,
            sync_mode: None,
            reader_threads: None,
            resize_algorithm: None,
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
        // TODO: fix me
        let db_directory = self
            .db_directory
            .unwrap_or_else(|| Cow::Borrowed(cuprate_blockchain_dir()));

        // Add the database filename to the directory.
        let db_file = {
            let mut db_file = db_directory.to_path_buf();
            db_file.push(DATABASE_DATA_FILENAME);
            Cow::Owned(db_file)
        };

        Config {
            db_directory,
            db_file,
            sync_mode: self.sync_mode.unwrap_or_default(),
            reader_threads: self.reader_threads.unwrap_or_default(),
            resize_algorithm: self.resize_algorithm.unwrap_or_default(),
        }
    }

    /// Set a custom database directory (and file) [`Path`].
    #[must_use]
    pub fn db_directory(mut self, db_directory: PathBuf) -> Self {
        self.db_directory = Some(Cow::Owned(db_directory));
        self
    }

    /// Tune the [`ConfigBuilder`] for the highest performing,
    /// but also most resource-intensive & maybe risky settings.
    ///
    /// Good default for testing, and resource-available machines.
    #[must_use]
    pub fn fast(mut self) -> Self {
        self.sync_mode = Some(SyncMode::Fast);
        self.reader_threads = Some(ReaderThreads::OnePerThread);
        self.resize_algorithm = Some(ResizeAlgorithm::default());
        self
    }

    /// Tune the [`ConfigBuilder`] for the lowest performing,
    /// but also least resource-intensive settings.
    ///
    /// Good default for resource-limited machines, e.g. a cheap VPS.
    #[must_use]
    pub fn low_power(mut self) -> Self {
        self.sync_mode = Some(SyncMode::default());
        self.reader_threads = Some(ReaderThreads::One);
        self.resize_algorithm = Some(ResizeAlgorithm::default());
        self
    }

    /// Set a custom [`SyncMode`].
    #[must_use]
    pub const fn sync_mode(mut self, sync_mode: SyncMode) -> Self {
        self.sync_mode = Some(sync_mode);
        self
    }

    /// Set a custom [`ReaderThreads`].
    #[must_use]
    pub const fn reader_threads(mut self, reader_threads: ReaderThreads) -> Self {
        self.reader_threads = Some(reader_threads);
        self
    }

    /// Set a custom [`ResizeAlgorithm`].
    #[must_use]
    pub const fn resize_algorithm(mut self, resize_algorithm: ResizeAlgorithm) -> Self {
        self.resize_algorithm = Some(resize_algorithm);
        self
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            // TODO: fix me
            db_directory: Some(Cow::Borrowed(cuprate_blockchain_dir())),
            sync_mode: Some(SyncMode::default()),
            reader_threads: Some(ReaderThreads::default()),
            resize_algorithm: Some(ResizeAlgorithm::default()),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Config
/// Database [`Env`](crate::Env) configuration.
///
/// This is the struct passed to [`Env::open`](crate::Env::open) that
/// allows the database to be configured in various ways.
///
/// For construction, either use [`ConfigBuilder`] or [`Config::default`].
///
// SOMEDAY: there's are many more options to add in the future.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    //------------------------ Database PATHs
    // These are private since we don't want
    // users messing with them after construction.
    /// The directory used to store all database files.
    ///
    /// By default, if no value is provided in the [`Config`]
    /// constructor functions, this will be [`cuprate_blockchain_dir`].
    ///
    // SOMEDAY: we should also support `/etc/cuprated.conf`.
    // This could be represented with an `enum DbPath { Default, Custom, Etc, }`
    pub(crate) db_directory: Cow<'static, Path>,
    /// The actual database data file.
    ///
    /// This is private, and created from the above `db_directory`.
    pub(crate) db_file: Cow<'static, Path>,

    /// Disk synchronization mode.
    pub sync_mode: SyncMode,

    /// Database reader thread count.
    pub reader_threads: ReaderThreads,

    /// Database memory map resizing algorithm.
    ///
    /// This is used as the default fallback, but
    /// custom algorithms can be used as well with
    /// [`Env::resize_map`](crate::Env::resize_map).
    pub resize_algorithm: ResizeAlgorithm,
}

impl Config {
    /// Create a new [`Config`] with sane default settings.
    ///
    /// The [`Config::db_directory`] will be [`cuprate_blockchain_dir`].
    ///
    /// All other values will be [`Default::default`].
    ///
    /// Same as [`Config::default`].
    ///
    /// ```rust
    /// use cuprate_database::{config::*, resize::*, DATABASE_DATA_FILENAME};
    /// use cuprate_helper::fs::*;
    ///
    /// let config = Config::new();
    ///
    /// assert_eq!(config.db_directory(), cuprate_blockchain_dir());
    /// assert!(config.db_file().starts_with(cuprate_blockchain_dir()));
    /// assert!(config.db_file().ends_with(DATABASE_DATA_FILENAME));
    /// assert_eq!(config.sync_mode, SyncMode::default());
    /// assert_eq!(config.reader_threads, ReaderThreads::default());
    /// assert_eq!(config.resize_algorithm, ResizeAlgorithm::default());
    /// ```
    pub fn new() -> Self {
        ConfigBuilder::default().build()
    }

    /// Return the absolute [`Path`] to the database directory.
    pub const fn db_directory(&self) -> &Cow<'_, Path> {
        &self.db_directory
    }

    /// Return the absolute [`Path`] to the database data file.
    pub const fn db_file(&self) -> &Cow<'_, Path> {
        &self.db_file
    }
}

impl Default for Config {
    /// Same as [`Config::new`].
    ///
    /// ```rust
    /// # use cuprate_database::config::*;
    /// assert_eq!(Config::default(), Config::new());
    /// ```
    fn default() -> Self {
        Self::new()
    }
}
