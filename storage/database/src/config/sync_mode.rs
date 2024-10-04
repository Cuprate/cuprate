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
///
/// # Sync vs Async
/// All invariants except [`SyncMode::Async`] & [`SyncMode::Fast`]
/// are `synchronous`, as in the database will wait until the OS has
/// finished syncing all the data to disk before continuing.
///
/// `SyncMode::Async` & `SyncMode::Fast` are `asynchronous`, meaning
/// the database will _NOT_ wait until the data is fully synced to disk
/// before continuing. Note that this doesn't mean the database itself
/// won't be synchronized between readers/writers, but rather that the
/// data _on disk_ may not be immediately synchronized after a write.
///
/// Something like:
/// ```rust,ignore
/// db.put("key", value);
/// db.get("key");
/// ```
/// will be fine, most likely pulling from memory instead of disk.
///
/// # SOMEDAY
/// Dynamic sync's are not yet supported.
///
/// Only:
///
/// - [`SyncMode::Safe`]
/// - [`SyncMode::Async`]
/// - [`SyncMode::Fast`]
///
/// are supported, all other variants will panic on [`crate::Env::open`].
#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SyncMode {
    /// Use [`SyncMode::Fast`] until fully synced,
    /// then use [`SyncMode::Safe`].
    ///
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

    #[default]
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

    /// Asynchrously sync to disk per transaction.
    ///
    /// This is the same as [`SyncMode::Safe`],
    /// but the syncs will be asynchronous, i.e.
    /// each transaction commit will sync to disk,
    /// but only eventually, not necessarily immediately.
    ///
    /// This maps to:
    /// - [`MDB_MAPASYNC`](http://www.lmdb.tech/doc/group__mdb__env.html#gab034ed0d8e5938090aef5ee0997f7e94)
    /// - [`redb::Durability::Eventual`](https://docs.rs/redb/1.5.0/redb/enum.Durability.html#variant.Eventual)
    Async,

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
    /// letting the OS decide when to flush data to disk[^1].
    ///
    /// This maps to:
    /// - [`MDB_NOSYNC`](http://www.lmdb.tech/doc/group__mdb__env.html#ga5791dd1adb09123f82dd1f331209e12e) + [`MDB_MAPASYNC`](http://www.lmdb.tech/doc/group__mdb__env.html#gab034ed0d8e5938090aef5ee0997f7e94)
    /// - [`redb::Durability::Eventual`](https://docs.rs/redb/1.5.0/redb/enum.Durability.html#variant.Eventual)
    ///
    /// [`monerod` reference](https://github.com/monero-project/monero/blob/7b7958bbd9d76375c47dc418b4adabba0f0b1785/src/blockchain_db/lmdb/db_lmdb.cpp#L1380-L1381).
    ///
    /// # Corruption
    /// In the case of a system crash, the database
    /// may become corrupted when using this option.
    ///
    ///
    /// [^1]: Semantically, this variant would actually map to
    /// [`redb::Durability::None`](https://docs.rs/redb/1.5.0/redb/enum.Durability.html#variant.None),
    /// however due to [`#149`](https://github.com/Cuprate/cuprate/issues/149),
    /// this is not possible. As such, when using the `redb` backend,
    /// transaction writes "should be persistent some time after `WriteTransaction::commit` returns."
    /// Thus, [`SyncMode::Async`] will map to the same `redb::Durability::Eventual` as [`SyncMode::Fast`].
    //
    // FIXME: we could call this `unsafe`
    // and use that terminology in the config file
    // so users know exactly what they are getting
    // themselves into.
    Fast,
}
