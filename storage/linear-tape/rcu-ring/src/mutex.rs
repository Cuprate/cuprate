use std::sync::atomic::{AtomicU32, Ordering};

use crate::atomic_wait::{wait, wake_one};

/// A mutex based on: https://marabos.nl/atomics/building-locks.html#mutex without any tied data.
pub struct Mutex<'a> {
    /// 0: unlocked
    /// 1: locked, no other threads waiting
    /// 2: locked, other threads waiting
    state: &'a AtomicU32,
}

impl<'a> Mutex<'a> {
    /// Creates a new mutex from a reference to an [`AtomicU32`].
    pub fn new(state: &'a AtomicU32) -> Self {
        Self { state }
    }

    /// lock the mutex, returning a [`MutexGuard`]
    pub fn lock(&self) -> MutexGuard<'_> {
        if self
            .state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // The lock was already locked. :(
            lock_contended(self.state);
        }
        MutexGuard { mutex: self }
    }
}

fn lock_contended(state: &AtomicU32) {
    let mut spin_count = 0;

    while state.load(Ordering::Relaxed) == 1 && spin_count < 100 {
        spin_count += 1;
        std::hint::spin_loop();
    }

    if state
        .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        return;
    }

    while state.swap(2, Ordering::Acquire) != 0 {
        wait(state, 2);
    }
}

/// A mutex guard that drops the lock when dropped.
pub struct MutexGuard<'a> {
    mutex: &'a Mutex<'a>,
}

impl Drop for MutexGuard<'_> {
    fn drop(&mut self) {
        if self.mutex.state.swap(0, Ordering::Release) == 2 {
            wake_one(self.mutex.state);
        }
    }
}
