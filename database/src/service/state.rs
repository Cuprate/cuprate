//! Database read thread-pool definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    sync::atomic::{AtomicU8, Ordering},
    sync::{Arc, RwLock, RwLockWriteGuard},
};

//---------------------------------------------------------------------------------------------------- Types
/// TODO
#[repr(u8)]
enum DatabaseState {
    /// TODO
    Ok = 0,
    /// TODO
    Resizing = 1,
    /// TODO
    ShuttingDown = 2,
}

impl From<u8> for DatabaseState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Ok,
            1 => Self::Resizing,
            2 => Self::ShuttingDown,
            _ => todo!(), // hmm what to do here
        }
    }
}

impl From<DatabaseState> for u8 {
    fn from(value: DatabaseState) -> Self {
        match value {
            DatabaseState::Ok => 0,
            DatabaseState::Resizing => 1,
            DatabaseState::ShuttingDown => 2,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Types
/// TODO
pub(super) struct DatadataStateHandleInner {
    /// TODO
    state: AtomicU8,
    /// TODO
    wait_until_resize_done: RwLock<()>,
}

/// TODO
pub(super) struct DatadataStateReader(DatadataStateHandleInner);

impl DatadataStateReader {
    /// TODO
    const fn new() -> Self {
        Self(DatadataStateHandleInner {
            state: AtomicU8::new(0),
            wait_until_resize_done: RwLock::new(()),
        })
    }

    /// TODO
    #[inline]
    fn get(&self) -> DatabaseState {
        self.0.state.load(Ordering::Acquire).into()
    }

    /// TODO
    #[inline]
    fn wait_until_resize_done(&self) {
        drop(self.0.wait_until_resize_done.read().unwrap());
    }
}

/// TODO
pub(super) struct DatadataStateWriter(DatadataStateHandleInner);

impl DatadataStateWriter {
    /// TODO
    fn store(&self, state: DatabaseState) {
        self.0.state.store(state.into(), Ordering::Release);
    }

    /// TODO
    fn get_mutual_exclusive_db_lock(&self) -> RwLockWriteGuard<'_, ()> {
        // New readers cannot enter.
        self.store(DatabaseState::Resizing);

        // Wait until older readers have left.
        self.0.wait_until_resize_done.write().unwrap()
    }
}
