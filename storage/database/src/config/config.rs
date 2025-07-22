//! The main [`Config`] struct, holding all configurable values.

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, num::NonZeroUsize, path::Path};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{config::SyncMode, constants::DATABASE_DATA_FILENAME, resize::ResizeAlgorithm};

//---------------------------------------------------------------------------------------------------- Constants
/// Default value for [`Config::reader_threads`].
///
/// ```rust
/// use cuprate_database::config::*;
/// assert_eq!(READER_THREADS_DEFAULT.get(), 126);
/// ```
pub const READER_THREADS_DEFAULT: NonZeroUsize = NonZeroUsize::new(126).unwrap();

//---------------------------------------------------------------------------------------------------- ConfigBuilder
/// Builder for [`Config`].
///
// SOMEDAY: there's are many more options to add in the future.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConfigBuilder {
    /// [`Config::db_directory`].
    db_directory: Cow<'static, Path>,

    /// [`Config::sync_mode`].
    sync_mode: Option<SyncMode>,

    /// [`Config::reader_threads`].
    reader_threads: Option<NonZeroUsize>,

    /// [`Config::resize_algorithm`].
    resize_algorithm: Option<ResizeAlgorithm>,
}

impl ConfigBuilder {
    /// Create a new [`ConfigBuilder`].
    ///
    /// [`ConfigBuilder::build`] can be called immediately
    /// after this function to use default values.
    pub const fn new(db_directory: Cow<'static, Path>) -> Self {
        Self {
            db_directory,
            sync_mode: None,
            reader_threads: Some(READER_THREADS_DEFAULT),
            resize_algorithm: None,
        }
    }

    /// Build into a [`Config`].
    ///
    /// # Default values
    /// - [`READER_THREADS_DEFAULT`] is used for [`Config::reader_threads`]
    /// - [`Default::default`] is used for all other values (except the `db_directory`)
    pub fn build(self) -> Config {
        // Add the database filename to the directory.
        let db_file = {
            let mut db_file = self.db_directory.to_path_buf();
            db_file.push(DATABASE_DATA_FILENAME);
            Cow::Owned(db_file)
        };

        Config {
            db_directory: self.db_directory,
            db_file,
            sync_mode: self.sync_mode.unwrap_or_default(),
            reader_threads: self.reader_threads.unwrap_or(READER_THREADS_DEFAULT),
            resize_algorithm: self.resize_algorithm.unwrap_or_default(),
        }
    }

    /// Set a custom database directory (and file) [`Path`].
    #[must_use]
    pub fn db_directory(mut self, db_directory: Cow<'static, Path>) -> Self {
        self.db_directory = db_directory;
        self
    }

    /// Tune the [`ConfigBuilder`] for the highest performing,
    /// but also most resource-intensive & maybe risky settings.
    ///
    /// Good default for testing, and resource-available machines.
    #[must_use]
    pub fn fast(mut self) -> Self {
        self.sync_mode = Some(SyncMode::Fast);
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
        self.resize_algorithm = Some(ResizeAlgorithm::default());
        self
    }

    /// Set a custom [`SyncMode`].
    #[must_use]
    pub const fn sync_mode(mut self, sync_mode: SyncMode) -> Self {
        self.sync_mode = Some(sync_mode);
        self
    }

    /// Set a custom [`Config::reader_threads`].
    #[must_use]
    pub const fn reader_threads(mut self, reader_threads: NonZeroUsize) -> Self {
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

//---------------------------------------------------------------------------------------------------- Config
/// Database [`Env`](crate::Env) configuration.
///
/// This is the struct passed to [`Env::open`](crate::Env::open) that
/// allows the database to be configured in various ways.
///
/// For construction, use [`ConfigBuilder`].
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
    ///
    /// Set the number of slots in the reader table.
    ///
    /// This is only used in LMDB, see
    /// [here](https://github.com/LMDB/lmdb/blob/b8e54b4c31378932b69f1298972de54a565185b1/libraries/liblmdb/mdb.c#L794-L799).
    ///
    /// By default, this value is [`READER_THREADS_DEFAULT`].
    pub reader_threads: NonZeroUsize,

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
    /// The [`Config::db_directory`] must be passed.
    ///
    /// All other values will be [`Default::default`].
    ///
    /// ```rust
    /// use cuprate_database::{config::*, resize::*, DATABASE_DATA_FILENAME};
    ///
    /// let tmp_dir = tempfile::tempdir().unwrap();
    /// let db_directory = tmp_dir.path().to_owned();
    /// let config = Config::new(db_directory.clone().into());
    ///
    /// assert_eq!(*config.db_directory(), db_directory);
    /// assert!(config.db_file().starts_with(db_directory));
    /// assert!(config.db_file().ends_with(DATABASE_DATA_FILENAME));
    /// assert_eq!(config.sync_mode, SyncMode::default());
    /// assert_eq!(config.reader_threads, READER_THREADS_DEFAULT);
    /// assert_eq!(config.resize_algorithm, ResizeAlgorithm::default());
    /// ```
    pub fn new(db_directory: Cow<'static, Path>) -> Self {
        ConfigBuilder::new(db_directory).build()
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
