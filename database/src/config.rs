//! Database [`Env`] configuration.
//!
//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::num::NonZeroUsize;

#[allow(unused_imports)] // docs
use crate::env::Env;

//---------------------------------------------------------------------------------------------------- Config
/// Database [`Env`] configuration.
///
/// This is the struct passed to [`Env::open`] that
/// allows the database to be configured in various ways.
#[derive(Copy, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Config {
    /// TODO
    pub sync_mode: SyncMode,

    /// TODO
    pub reader_threads: ReaderThreads,
}

impl Config {
    /// TODO
    pub const fn new() -> Self {
        Self {
            sync_mode: SyncMode::Safe,
            reader_threads: ReaderThreads::OnePerThread,
        }
    }

    /// TODO
    pub const fn fast() -> Self {
        Self {
            sync_mode: SyncMode::Fastest,
            reader_threads: ReaderThreads::OnePerThread,
        }
    }

    /// TODO
    pub const fn low_power() -> Self {
        Self {
            sync_mode: SyncMode::Safe,
            reader_threads: ReaderThreads::One,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
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
