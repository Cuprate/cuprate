//! Database-related state passed to all reader threads and the writer.
//!
//! This file contains some data structures and functions
//! for some small but important signals/state that is
//! used for communication between the `service` reader
//! thread-pool and the writer.
//!
//! None of this is public, all of this is
//! pretty much an "implementation detail".

//---------------------------------------------------------------------------------------------------- Import
use std::sync::{Arc, RwLock, RwLockWriteGuard};

//---------------------------------------------------------------------------------------------------- DatabaseState
/// All of the state and machinery needed for the reader thread-pool
/// and writer thread to be able to communicate to each other what they need to.
///
/// This is the "raw" version of the data, to be wrapped in an `Arc`.
/// Any owner of this struct has write access, so readers get a special
/// `DatabaseStateReader` type instead.
///
/// # Private
/// The fields of this struct don't necessarily need to
/// be private to this file, but it's nicer for helper functions
/// to be created/used by the readers/writer instead.
#[derive(Debug)]
pub(super) struct DatabaseState {
    /// Atomic representation of a `DatabaseSignal`.
    signal: AtomicDatabaseSignal,

    /// Lock representing mutual exclusive access to the database.
    ///
    /// # Writer
    /// Upon situations where the writer needs mutual exclusive
    /// access to the database (e.g. resizing), it will:
    ///
    /// 1. Set the `signal` such that _future_ readers do not enter
    /// 2. Hang on this lock acquiring a `write` handle such that
    ///    we wait until all _current_ readers have cleared
    ///
    /// Once we have this lock, we know that we (as the single writer) have
    /// "exclusive access" to the database, and that new readers cannot come in.
    ///
    /// # Reader
    /// Why are there 2 semaphores here though? Can't we just have the lock?
    ///
    /// 1. Atomically loading `DatabaseSignal::Ok` allows the reader to enter
    ///    the fast path and continue execution much faster than acquiring a lock
    ///    (the difference must be absolutely tiny, but this is happening
    ///    on _every_ read operation so it adds up)
    /// 2. We need a cheap way to sleep the readers, and make them wait
    ///    until the writer is done with the mutual exclusion.
    /// 3. We need more than just a lock anyway, we need "signals".
    ///
    /// Reader's on _every_ transaction will do the following:
    /// 1. Atomically access `signal`
    /// 2. If `Ok`, continue the transaction - we're all good
    /// 3a. If `Resizing`, call `db_lock.read()`. This allows us
    ///    (the reader thread) to sleep until the writer is done
    /// 3b. Continue the transaction as normal
    /// 4. If `Shutdown`, run shutdown code and exit.
    ///
    /// Note how Reader's don't touch `db_lock` on the `Ok` path,
    /// they just need to do 1 atomic fetch.
    ///
    /// # Invariant
    /// This whole `DatabaseState` system _may_ depend on
    /// the fact that there's only 1 writer thread.
    db_lock: RwLock<()>,
}

impl DatabaseState {
    /// Initialize a new reader/write [`DatabaseState`] pair.
    #[cold]
    #[inline(never)] // Called once per [`crate::service::init()`]
    pub(super) fn new() -> (DatabaseStateReader, Arc<Self>) {
        let writer = Arc::new(Self {
            signal: AtomicDatabaseSignal::new(DatabaseSignal::Ok),
            db_lock: RwLock::new(()),
        });

        let reader = DatabaseStateReader(Arc::clone(&writer));

        (reader, writer)
    }

    /// TODO
    pub(super) fn store_signal(&self, signal: DatabaseSignal) {
        // INVARIANT: must be `Release` to match the reader's `Acquire` load.
        self.signal.store(signal);
    }

    /// TODO
    pub(super) fn db_lock(&self) -> RwLockWriteGuard<'_, ()> {
        // New readers cannot enter.
        self.store_signal(DatabaseSignal::Resizing);

        // Wait until older readers have left.
        self.db_lock.write().unwrap()
    }
}

/// Is just a [`DatabaseState`] but only
/// has read-related functions implemented.
///
/// We could just pass around `DatabaseState` around
/// to everyone and "not" call the write functions from the
/// reader thread but why not just use the type system.
///
/// Being explicit here lessens the chance of someone in the
/// future (most likely me) and messing things up accidently.
#[derive(Clone, Debug)]
pub(super) struct DatabaseStateReader(Arc<DatabaseState>);

impl DatabaseStateReader {
    /// TODO
    #[inline]
    pub(super) fn get_signal(&self) -> DatabaseSignal {
        // INVARIANT: must be `Acquire` to match the writer's `Release` store.
        self.0.signal.load()
    }

    /// TODO
    #[inline]
    pub(super) fn wait_until_resize_done(&self) {
        drop(self.0.db_lock.read().unwrap());
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseSignal
/// An enumeration of the possible "states" the database is in.
///
/// When a reader receives a request, it will peek
/// at this state before entering the transaction.
///
/// Why? Because the database might be in any of the
/// non-`Ok` states laid out below, and the reader
/// thread must react accordingly.
#[repr(u8)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum DatabaseSignal {
    /// All is OK - proceed with whatever database operations you want.
    #[default]
    Ok = 0,

    /// The database is in the process of resizing,
    /// if you're a Reader, wait a little while!
    Resizing = 1,

    /// The database is shutting down.
    /// If you're a Reader, exit the thread!
    ShuttingDown = 2,
}

/// An atomic [`DatabaseSignal`].
type AtomicDatabaseSignal = crossbeam::atomic::AtomicCell<DatabaseSignal>;
/// Compile time assertion [`AtomicDatabaseSignal`] is lock free.
const _: () = assert!(
    AtomicDatabaseSignal::is_lock_free(),
    "AtomicDatabaseSignal is not lock free!",
);
