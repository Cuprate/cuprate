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
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

//---------------------------------------------------------------------------------------------------- SyncMode
/// Disk synchronization mode.
///
/// This controls how/when the database syncs its data to disk.
///
/// Regardless of the variant chosen, dropping [`Env`](crate::Env)
/// will always cause it to fully sync to disk.
#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SyncMode {
    /// Use [`SyncMode::Fast`] until fully synced,
    /// then use [`SyncMode::Safe`].
    ///
    /// # TODO
    /// This is not implemented internally and has the same behavior as [`SyncMode::Fast`].
    //
    // # SOMEDAY: how to implement this?
    // ref: <https://github.com/monero-project/monero/issues/1463>
    // monerod-solution: <https://github.com/monero-project/monero/pull/1506>
    // cuprate-issue: <https://github.com/Cuprate/cuprate/issues/78>
    //
    // We could:
    // ```rust,ignore
    // if current_db_block <= top_block.saturating_sub(N) {
    //     // don't sync()
    // } else {
    //     // sync()
    // }
    // ```
    // where N is some threshold we pick that is _close_ enough
    // to being synced where we want to start being safer.
    //
    // Essentially, when we are in a certain % range of being finished,
    // switch to safe mode, until then, go fast.
    FastThenSafe,

    /// Fully sync to disk per transaction.
    ///
    /// Every database transaction commit will
    /// fully sync all data to disk, _synchronously_,
    /// so the database (writer) halts until synced.
    ///
    /// This is expected to be very slow.
    ///
    /// This maps to:
    /// - LMDB without any special sync flags
    /// - [`redb::Durability::Immediate`](https://docs.rs/redb/1.5.0/redb/enum.Durability.html#variant.Immediate)
    Safe,

    #[default]
    /// Only flush at database shutdown.
    ///
    /// This is the fastest, yet unsafest option.
    ///
    /// It will cause the database to never _actively_ sync,
    /// letting the OS decide when to flush data to disk[^1].
    ///
    /// This maps to:
    /// - [`MDB_NOSYNC | MDB_WRITEMAP | MDB_MAPASYNC`](https://github.com/monero-project/monero/blob/90359e31fd657251cb357ecba02c4de2442d1b5c/src/blockchain_db/lmdb/db_lmdb.cpp#L1444)
    /// - [`redb::Durability::Eventual`](https://docs.rs/redb/1.5.0/redb/enum.Durability.html#variant.Eventual)
    ///
    /// # Default
    /// This is the default [`SyncMode`].
    /// ```rust
    /// use cuprate_database::config::SyncMode;
    ///
    /// assert_eq!(SyncMode::default(), SyncMode::Fast);
    /// ```
    ///
    /// # Corruption
    /// In the case of a system crash, the database
    /// may become corrupted when using this option.
    ///
    /// [^1]: Semantically, this variant would actually map to
    /// [`redb::Durability::None`](https://docs.rs/redb/1.5.0/redb/enum.Durability.html#variant.None),
    /// however due to [`#149`](https://github.com/Cuprate/cuprate/issues/149),
    /// this is not possible. As such, when using the `redb` backend,
    /// transaction writes "should be persistent some time after `WriteTransaction::commit` returns."
    Fast,
}
