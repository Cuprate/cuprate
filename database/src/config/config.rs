//! Database [`Env`](crate::Env) configuration.
//!
//! This module contains the main [`Config`]uration struct
//! for the database [`Env`](crate::Env)ironment, and data
//! structures related to any configuration setting.
//!
//! These configurations are processed at runtime, meaning
//! the `Env` can/will dynamically adjust its behavior
//! based on these values.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_helper::fs::cuprate_database_dir;

use crate::{
    config::{ReaderThreads, SyncMode},
    constants::DATABASE_DATA_FILENAME,
    resize::ResizeAlgorithm,
};

//---------------------------------------------------------------------------------------------------- Config
/// Database [`Env`](crate::Env) configuration.
///
/// This is the struct passed to [`Env::open`](crate::Env::open) that
/// allows the database to be configured in various ways.
///
/// TODO: there's probably more options to add.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    //------------------------ Database PATHs
    // These are private since we don't want
    // users messing with them after construction.
    /// The directory used to store all database files.
    ///
    /// By default, if no value is provided in the [`Config`]
    /// constructor functions, this will be [`cuprate_database_dir`].
    ///
    /// TODO: we should also support `/etc/cuprated.conf`.
    /// This could be represented with an `enum DbPath { Default, Custom, Etc, }`
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
    /// Private function to acquire [`Config::db_file`]
    /// from the user provided (or default) [`Config::db_directory`].
    ///
    /// As the database data file PATH is just the directory + the filename,
    /// we only need the directory from the user/Config, and can add it here.
    fn return_db_dir_and_file(
        db_directory: Option<PathBuf>,
    ) -> (Cow<'static, Path>, Cow<'static, Path>) {
        // INVARIANT: all PATH safety checks are done
        // in `helper::fs`. No need to do them here.
        let db_directory =
            db_directory.map_or_else(|| Cow::Borrowed(cuprate_database_dir()), Cow::Owned);

        // Add the database filename to the directory.
        let mut db_file = db_directory.to_path_buf();
        db_file.push(DATABASE_DATA_FILENAME);

        (db_directory, Cow::Owned(db_file))
    }

    /// Create a new [`Config`] with sane default settings.
    ///
    /// # `db_directory`
    /// If this is `Some`, it will be used as the
    /// directory that contains all database files.
    ///
    /// If `None`, it will use the default directory [`cuprate_database_dir`].
    pub fn new(db_directory: Option<PathBuf>) -> Self {
        let (db_directory, db_file) = Self::return_db_dir_and_file(db_directory);
        Self {
            db_directory,
            db_file,
            sync_mode: SyncMode::default(),
            reader_threads: ReaderThreads::OnePerThread,
            resize_algorithm: ResizeAlgorithm::default(),
        }
    }

    /// Create a [`Config`] with the highest performing,
    /// but also most resource-intensive & maybe risky settings.
    ///
    /// Good default for testing, and resource-available machines.
    ///
    /// # `db_directory`
    /// If this is `Some`, it will be used as the
    /// directory that contains all database files.
    ///
    /// If `None`, it will use the default directory [`cuprate_database_dir`].
    pub fn fast(db_directory: Option<PathBuf>) -> Self {
        let (db_directory, db_file) = Self::return_db_dir_and_file(db_directory);
        Self {
            db_directory,
            db_file,
            sync_mode: SyncMode::Fast,
            reader_threads: ReaderThreads::OnePerThread,
            resize_algorithm: ResizeAlgorithm::default(),
        }
    }

    /// Create a [`Config`] with the lowest performing,
    /// but also least resource-intensive settings.
    ///
    /// Good default for resource-limited machines, e.g. a cheap VPS.
    ///
    /// # `db_directory`
    /// If this is `Some`, it will be used as the
    /// directory that contains all database files.
    ///
    /// If `None`, it will use the default directory [`cuprate_database_dir`].
    pub fn low_power(db_directory: Option<PathBuf>) -> Self {
        let (db_directory, db_file) = Self::return_db_dir_and_file(db_directory);
        Self {
            db_directory,
            db_file,
            sync_mode: SyncMode::default(),
            reader_threads: ReaderThreads::One,
            resize_algorithm: ResizeAlgorithm::default(),
        }
    }

    /// Return the absolute [`Path`] to the database directory.
    ///
    /// This will be the `db_directory` given
    /// (or default) during [`Config`] construction.
    pub const fn db_directory(&self) -> &Cow<'_, Path> {
        &self.db_directory
    }

    /// Return the absolute [`Path`] to the database data file.
    ///
    /// This will be based off the `db_directory` given
    /// (or default) during [`Config`] construction.
    pub const fn db_file(&self) -> &Cow<'_, Path> {
        &self.db_file
    }
}

impl Default for Config {
    /// Same as `Self::new(None)`.
    ///
    /// ```rust
    /// # use cuprate_database::config::*;
    /// assert_eq!(Config::default(), Config::new(None));
    /// ```
    fn default() -> Self {
        Self::new(None)
    }
}
