//! Database [`Env`] configuration.
//!
//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, num::NonZeroUsize, path::Path};

#[allow(unused_imports)] // docs
use crate::env::Env;

//---------------------------------------------------------------------------------------------------- Config
/// Database [`Env`] configuration.
///
/// This is the struct passed to [`Env::open`] that
/// allows the database to be configured in various ways.
///
/// TODO: there's probably more options to add.
#[derive(Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Config {
    /// TODO
    pub db_directory: Cow<'static, Path>,

    /// TODO
    pub sync_mode: SyncMode,

    /// Database reader thread count.
    pub reader_threads: ReaderThreads,
}

impl Config {
    /// TODO
    pub fn new<P: AsRef<Path>>(db_directory: Option<P>) -> Self {
        Self {
            db_directory: db_directory.map_or(todo!(), |p| Cow::Owned(p.as_ref().to_path_buf())),
            sync_mode: SyncMode::Safe,
            reader_threads: ReaderThreads::OnePerThread,
        }
    }

    /// TODO
    pub fn fast<P: AsRef<Path>>(db_directory: Option<P>) -> Self {
        Self {
            db_directory: db_directory.map_or(todo!(), |p| Cow::Owned(p.as_ref().to_path_buf())),
            sync_mode: SyncMode::Fastest,
            reader_threads: ReaderThreads::OnePerThread,
        }
    }

    /// TODO
    pub fn low_power<P: AsRef<Path>>(db_directory: Option<P>) -> Self {
        Self {
            db_directory: db_directory.map_or(todo!(), |p| Cow::Owned(p.as_ref().to_path_buf())),
            sync_mode: SyncMode::Safe,
            reader_threads: ReaderThreads::One,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(Some("TODO: default cuprate database dir"))
    }
}

//---------------------------------------------------------------------------------------------------- SyncMode
/// TODO
#[derive(Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum SyncMode {
    /// Fully sync to disk per transaction.
    #[default]
    Safe,

    /// Asynchronously sync, only flush at database shutdown.
    Fastest,
}

//---------------------------------------------------------------------------------------------------- ReaderThreads
/// TODO
#[derive(Copy, Clone, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum ReaderThreads {
    #[default]
    /// TODO
    OnePerThread,

    /// TODO
    One,

    /// TODO
    Number(NonZeroUsize),

    /// TODO
    ///
    /// # Invariant
    /// Must be `0.0..=1.0`.
    Percent(f32),
}

impl ReaderThreads {
    /// TODO
    // # Invariant
    // LMDB will error if we input zero, so don't allow that.
    // <https://github.com/LMDB/lmdb/blob/b8e54b4c31378932b69f1298972de54a565185b1/libraries/liblmdb/mdb.c#L4687>
    pub fn as_threads(&self) -> NonZeroUsize {
        let total_threads = cuprate_helper::thread::threads();

        match self {
            Self::OnePerThread => total_threads,
            Self::One => NonZeroUsize::MIN,
            Self::Number(n) => std::cmp::min(*n, total_threads),

            // We handle the casting loss.
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            Self::Percent(f) => {
                if !f.is_normal() || !(0.0..=1.0).contains(f) {
                    return total_threads;
                }

                let thread_percent = (total_threads.get() as f32) * f;
                let Some(threads) = NonZeroUsize::new(thread_percent as usize) else {
                    return total_threads;
                };

                std::cmp::min(threads, total_threads)
            }
        }
    }
}
