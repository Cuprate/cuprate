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
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use cuprate_helper::fs::cuprate_database_dir;

use crate::{constants::DATABASE_FILENAME, resize::ResizeAlgorithm};

//---------------------------------------------------------------------------------------------------- Config
/// Database [`Env`](crate::Env) configuration.
///
/// This is the struct passed to [`Env::open`](crate::Env::open) that
/// allows the database to be configured in various ways.
///
/// TODO: there's probably more options to add.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Config {
    //------------------------ Database PATHs
    // These are private since we don't want
    // users messing with them after construction.
    /// The directory used to store all database files.
    ///
    /// By default, if no value is provided in the [`Config`]
    /// constructor functions, this will be [`cuprate_database_dir`].
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
        db_file.push(DATABASE_FILENAME);

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
            sync_mode: SyncMode::FastThenSafe,
            reader_threads: ReaderThreads::OnePerThread,
            resize_algorithm: ResizeAlgorithm::new(),
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
            resize_algorithm: ResizeAlgorithm::new(),
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
            sync_mode: SyncMode::FastThenSafe,
            reader_threads: ReaderThreads::One,
            resize_algorithm: ResizeAlgorithm::new(),
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

//---------------------------------------------------------------------------------------------------- SyncMode
/// Disk synchronization mode.
///
/// This controls how/when the database syncs its data to disk.
///
/// Regardless of the variant chosen, dropping [`Env`](crate::Env)
/// will always cause it to fully sync to disk.
///
/// # Sync vs Async
/// All invariants except [`SyncMode::Fast`] are `synchronous`,
/// as in the database will wait until the sync is finished before continuing.
///
/// `SyncMode::Fast` is `asynchronous`, meaning it will _NOT_
/// wait until the sync is done before continuing. It will immediately
/// move onto the next operation even if the database/OS has not responded.
#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum SyncMode {
    /// Use [`SyncMode::Fast`] until fully synced,
    /// then use [`SyncMode::Safe`].
    ///
    /// TODO: We could not bother with this and just implement
    /// batching, which was the solution to slow syncs with `Safe`.
    /// ref: <https://github.com/monero-project/monero/issues/1463>
    /// solution: <https://github.com/monero-project/monero/pull/1506>
    ///
    /// TODO: this could be implemented as:
    /// ```rust,ignore
    /// if current_db_block <= top_block.saturating_sub(N) {
    ///     // don't sync()
    /// } else {
    ///     // sync()
    /// }
    /// ```
    /// where N is some threshold we pick that is _close_ enough
    /// to being synced where we want to start being safer.
    ///
    /// Essentially, when we are in a certain % range of being finished,
    /// switch to safe mode, until then, go fast.
    #[default]
    FastThenSafe,

    /// Fully sync to disk per transaction.
    ///
    /// Every database transaction commit will
    /// fully sync all data to disk, _synchronously_,
    /// so the database halts until synced.
    ///
    /// This is expected to be very slow.
    Safe,

    /// Fully sync to disk after we cross this transaction threshold.
    ///
    /// After committing [`usize`] amount of database
    /// transactions, it will be sync to disk.
    ///
    /// `0` behaves the same as [`SyncMode::Safe`], and a ridiculously large
    /// number like `usize::MAX` is practically the same as [`SyncMode::Fast`].
    Threshold(usize),

    /// Only flush at database shutdown.
    ///
    /// This is the fastest, yet unsafest option.
    ///
    /// It will cause the database to never _actively_ sync,
    /// letting the OS decide when to flush data to disk.
    ///
    /// # Corruption
    /// In the case of a system crash, the database
    /// may become corrupted when using this option.
    //
    // TODO: we could call this `unsafe`
    // and use that terminology in the config file
    // so users know exactly what they are getting
    // themselves into.
    Fast,
}

//---------------------------------------------------------------------------------------------------- ReaderThreads
/// Amount of database reader threads to spawn.
///
/// This controls how many reader thread [`crate::service`]'s
/// thread-pool will spawn to receive and send requests/responses.
///
/// It will always be at least 1, up until the amount of threads on the machine.
///
/// The main function used to extract an actual
/// usable thread count out of this is [`ReaderThreads::as_threads`].
#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum ReaderThreads {
    #[default]
    /// Spawn 1 reader thread per available thread on the machine.
    ///
    /// For example, a `16-core, 32-thread` Ryzen 5950x will
    /// spawn `32` reader threads using this setting.
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
    /// # use cuprate_database::config::*;
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
    /// # use cuprate_database::config::*;
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
    /// use cuprate_database::config::ReaderThreads as Rt;
    ///
    /// let total_threads: std::num::NonZeroUsize =
    ///     cuprate_helper::thread::threads();
    ///
    /// assert_eq!(Rt::OnePerThread.as_threads(), total_threads);
    ///
    /// assert_eq!(Rt::One.as_threads().get(), 1);
    ///
    /// assert_eq!(Rt::Number(0).as_threads(), total_threads);
    /// assert_eq!(Rt::Number(1).as_threads().get(), 1);
    /// assert_eq!(Rt::Number(usize::MAX).as_threads(), total_threads);
    ///
    /// assert_eq!(Rt::Percent(0.01).as_threads().get(), 1);
    /// assert_eq!(Rt::Percent(0.0).as_threads(), total_threads);
    /// assert_eq!(Rt::Percent(1.0).as_threads(), total_threads);
    /// assert_eq!(Rt::Percent(f32::NAN).as_threads(), total_threads);
    /// assert_eq!(Rt::Percent(f32::INFINITY).as_threads(), total_threads);
    /// assert_eq!(Rt::Percent(f32::NEG_INFINITY).as_threads(), total_threads);
    ///
    /// // Percentage only works on more than 1 thread.
    /// if total_threads.get() > 1 {
    ///     assert_eq!(
    ///         Rt::Percent(0.5).as_threads().get(),
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
            #[allow(
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
    /// If `value` is `0`, this will return [`ReturnThreads::OnePerThread`].
    fn from(value: T) -> Self {
        let u: usize = value.into();
        if u == 0 {
            Self::OnePerThread
        } else {
            Self::Number(u)
        }
    }
}
