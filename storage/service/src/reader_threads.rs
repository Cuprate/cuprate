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
use rayon::ThreadPool;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::Arc;

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
/// Amount of database reader threads to spawn when using [`service`](crate::service).
///
/// This controls how many reader thread `service`'s
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
