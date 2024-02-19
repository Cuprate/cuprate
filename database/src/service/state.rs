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
use std::{
    sync::atomic::{AtomicU8, Ordering},
    sync::{Arc, RwLock, RwLockWriteGuard},
};

//---------------------------------------------------------------------------------------------------- DatabaseState
/// All of the state and machinery needed for the reader thread-pool
/// and writer thread to be able to communicate to each what they need to.
///
/// This is shared information/signals between
/// the reader thread-pool and writer.
///
/// This is the "raw" version of the data, to be wrapped in an `Arc`.
/// Any owner of this struct has write access, so readers get a special
/// `DatabaseStateReader` type.
///
/// # Invariant
/// The fields of this struct are private on purpose.
///
/// We must maintain the invariant:
/// - `signal` is a valid `u8` representation of `DatabaseSignal`
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
    /// 1. `DatabaseSignal` does a whole bunch of stuff and signals the _why_
    /// 2. Atomically loading `DatabaseSignal::Ok` allows the reader to enter
    ///    the happy path and continue execution much cheaper than acquiring a lock
    ///    (the difference must be absolutely tiny, but this is happening
    ///    on _every_ read operation so it adds up)
    /// 3. We need a cheap way to sleep the readers, and wait for them to
    ///    "re-enter" once the writer is done with whatever they needed the
    ///    mutual exclusion for. This lock provide that.
    ///
    /// Reader's on _every_ transaction will do the following:
    /// 1. Atomically access `signal`
    /// 2. If `Ok`, continue the transaction - we're all good
    /// 3a. If `Resizing`, call `db_lock.read()`. This allows us
    ///    (the reader thread) to sleep until the writer is done
    /// 3b. Continue the transaction as normal
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
            signal: AtomicDatabaseSignal::new(),
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
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum DatabaseSignal {
    /// All is OK - proceed with whatever database operations you want.
    Ok = 0,

    /// The database is in the process of resizing,
    /// if you're a Reader, wait a little while!
    Resizing = 1,

    /// The database is shutting down.
    /// If you're a Reader, exit the thread!
    ShuttingDown = 2,
}

/// An atomic [`DatabaseSignal`].
///
/// This type upholds the invariant that the internal [`AtomicU8`]
/// is within the valid [`u8`] range of `DatabaseSignal`.
///
/// Private, purely used in this file.
#[repr(transparent)]
#[derive(Debug)]
struct AtomicDatabaseSignal(AtomicU8);

impl AtomicDatabaseSignal {
    /// Create a new [`AtomicDatabaseSignal`] set with [`DatabaseSignal::Ok`].
    const fn new() -> Self {
        Self(AtomicU8::new(0))
    }

    /// Get the inner [`DatabaseSignal`].
    ///
    /// Uses [`Ordering::Acquire`].
    fn load(&self) -> DatabaseSignal {
        match self.0.load(Ordering::Acquire) {
            0 => DatabaseSignal::Ok,
            1 => DatabaseSignal::Resizing,
            2 => DatabaseSignal::ShuttingDown,
            _ => unreachable!(),
        }
    }

    /// Atomicall store a [`DatabaseSignal`].
    ///
    /// Uses [`Ordering::Release`].
    fn store(&self, signal: DatabaseSignal) {
        self.0.store(
            match signal {
                DatabaseSignal::Ok => 0,
                DatabaseSignal::Resizing => 1,
                DatabaseSignal::ShuttingDown => 2,
            },
            Ordering::Release,
        );
    }
}
