use std::sync::atomic::{fence, AtomicU32, Ordering};

use crate::atomic_wait::{wait, wake_all};

/// The state of a data-slot.
///
/// Tracks the number of readers and if the data has been marked as old.
pub(crate) struct DataSlotState<'a>(&'a AtomicU32);

/// A bit-flag for if the data is old.
const DATA_SLOT_OLD: u32 = 1 << 31;
/// A bit-flag for if a writer is waiting.
const WRITER_WAITING: u32 = 1 << 30;

/// The mask to `&` with to get the amount of readers using this slot.
const READER_COUNT_MASK: u32 = !(DATA_SLOT_OLD | WRITER_WAITING);

impl DataSlotState<'_> {
    /// Creates a new [`DataSlotState`] from a ptr.
    ///
    /// # Safety
    ///
    /// see: [`AtomicU32::from_ptr`]
    pub(crate) unsafe fn from_u8_ptr(ptr: *mut u8) -> Self {
        let ptr = ptr.cast::<u32>();

        assert!(ptr.cast::<AtomicU32>().is_aligned());

        // Safety: the callers must ensure the requirements, hence this function is `unsafe`.
        unsafe { Self(AtomicU32::from_ptr(ptr)) }
    }

    /// Add a new reader to the count, returning if a reader has been added ([`true`]) or ([`false`])
    /// if the data-slot is old and the pointer to the slot should be re-read.
    pub(crate) fn check_add_reader(&self) -> bool {
        // We can use a `Relaxed` ordering here as we `Acquire` in the next call
        let mut state = self.0.load(Ordering::Relaxed);

        loop {
            // Check if the data is old.
            if state & DATA_SLOT_OLD == DATA_SLOT_OLD {
                return false;
            }

            // Check we have room for another reader.
            if state & READER_COUNT_MASK == READER_COUNT_MASK - 1 {
                panic!("Too many readers");
            }

            // We use `Acquire` here as I think it might be possible for a thread to see
            // the old data-slot ptr that points to the same slot as a writer just updated to. This
            // `Acquire` then synchronizes with the `Release` in clear old.
            match self.0.compare_exchange_weak(
                state,
                state + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(new) => {
                    state = new;
                    std::hint::spin_loop();
                }
            }
        }
    }

    /// Mark this data-slot as old.
    pub(crate) fn mark_old(&self) {
        // Relaxed as we don't need any ordering requirements here.
        self.0.fetch_or(DATA_SLOT_OLD, Ordering::Relaxed);
    }

    /// Clear the old mark for this data-slot, returning the state to its default.
    pub(crate) fn clear_old(&self) {
        // Release for the reason in `check_add_reader`
        self.0.store(0, Ordering::Release);
    }

    /// Waits for all the readers to finish their read operation and drop their handle to this
    /// data-slot.
    pub(crate) fn wait_for_readers(&self) {
        let mut i = 0;
        let mut old = self.0.fetch_or(WRITER_WAITING, Ordering::Relaxed);
        loop {
            if old & READER_COUNT_MASK == 0 {
                // We have to use `Acquire` here to synchronise with the readers `Release` when dropping
                // the count to prevent writing to data they are reading.
                // We use a `fence` instead of using `Acquire` for every load for efficiency.
                fence(Ordering::Acquire);
                return;
            }

            if i > 100 {
                wait(self.0, old | WRITER_WAITING);
            } else {
                std::hint::spin_loop();
            }

            i += 1;
            old = self.0.load(Ordering::Relaxed);
        }
    }

    /// Drop the readers count by 1.
    pub fn remove_reader(&self) {
        // `Release` ordering so this can't be reordered to before a data access.
        let old = self.0.fetch_sub(1, Ordering::Release);

        // if this is that last reader and a writer is waiting, wake
        if old & READER_COUNT_MASK == 1 && old & WRITER_WAITING == WRITER_WAITING {
            wake_all(self.0)
        }
    }
}
