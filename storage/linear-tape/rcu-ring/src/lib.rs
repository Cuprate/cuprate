//! # Rcu-Tape
//!
//! A sort-of implementation of a read-copy-update synchronization mechanism: <https://en.wikipedia.org/wiki/Read-copy-update>
//!
//! This crate exposes and [`RcuRing`] type, which implements an RCU mechanism over a raw byte buffer,
//! treating the byte buffer as a contiguous ring of slots to use for the data (with some header bytes
//! at the start).
//!
//! It supports multiple readers and a single writer, writing to the next slot. The number of slots
//! is configurable, the more slots you have, the more space is required, but the less waiting writers
//! will have to do to make sure all readers finish reading old data.
//!
//! This is only a sort-of RCU as technically readers could wait on a writer while a writer is doing a commit.
//! If the reader reads the old ptr but sees the marker that the slot is old it will keep trying the ptr until
//! it sees the new value. In practice this can be avoided with enough slots.
//!
//! This is a low-level synchronization primitive.
use std::sync::atomic::{AtomicU32, Ordering};

pub(crate) mod atomic_wait;
mod mutex;
mod state;

use mutex::{Mutex, MutexGuard};
use state::DataSlotState;

/// The size of the RCU header.
///
/// The header contains the writer mutex and the current slot idx.
const HEADER_LEN: usize = 8;

/// A handle to a slot in the RcuRing.
///
/// This handle holds a sort of read guard to prevent a writer writing to this slot.
pub struct DataHandle<'a> {
    data_slot_state: DataSlotState<'a>,
    /// The data in this slot.
    ///
    /// This is guaranteed to be at least 8-byte aligned.
    pub data: &'a [u8],
}

impl Drop for DataHandle<'_> {
    fn drop(&mut self) {
        self.data_slot_state.remove_reader();
    }
}

/// A write guard, which allows updating the RcuRing.
pub struct WriteGuard<'a> {
    _mutex_guard: MutexGuard<'a>,
    /// The [`DataSlotState`] of the current slot used for readers.
    current_data_slot_state: DataSlotState<'a>,
    /// The [`AtomicU32`] which holds the index of the data-slot.
    current_data_slot_idx: &'static AtomicU32,
    /// The index of the slot we are writing to.
    next_data_slot_idx: u32,
    /// The [`DataSlotState`] of the slot we are writing to.
    next_data_slot_state: DataSlotState<'a>,
    /// The data of the slot we are writing to.
    data: &'a mut [u8],
}

impl WriteGuard<'_> {
    /// Returns a mutable pointer to the data in the slot we are writing to.
    ///
    /// This is guaranteed to be at least 8-byte aligned.
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.data
    }

    /// The current index up-to-date readers will see.
    pub fn current_data_slot_idx(&self) -> usize {
        self.current_data_slot_idx.load(Ordering::Relaxed) as usize
    }

    /// Push an update to the RcuRing.
    ///
    /// You should not use this guard after calling this function.
    pub fn push_update(&mut self) {
        self.data = &mut [];
        self.current_data_slot_state.mark_old();
        self.next_data_slot_state.clear_old();
        self.current_data_slot_idx
            .store(self.next_data_slot_idx, Ordering::Release);
    }
}

/// A RCU (read-copy-update) Ring.
///
/// This type is backed by a single contiguous byte buffer and allows multiple readers to read a value
/// and a single writer to concurrently be writing data, which can then be atomically flushed to the
/// ring for new readers to see.
///
/*
Current format:

    Header:
        writer_mutex: (4 bytes)
        current_data_slot_idx: (4 bytes)
    Ring (data_slots_ring_len amount):
        data_slot_state: (4 bytes)
        last_op: (4 bytes)
        data: (data_len bytes)
 */
pub struct RcuRing {
    /// The length of the data in a single data-slot.
    data_len: usize,
    /// The buffer which holds all slots.
    buffer: *mut u8,
    /// A mutex for a writer.
    writer_mutex: Mutex<'static>,
    /// The index of the current slot for readers.
    current_data_slot_idx: &'static AtomicU32,
    /// The amount of data-slots.
    data_slots_ring_len: usize,
}

// We have internal synchronisation to make this safe.
unsafe impl Send for RcuRing {}
unsafe impl Sync for RcuRing {}

impl RcuRing {
    /// Creates a new [`RcuRing`].
    ///
    /// `data_slots_ring_len` must be at least 2.
    ///
    /// # Safety
    ///
    /// - `buffer` must be 8-byte aligned and have no other code with any references to it (multiple instances of [`RcuRing`] is ok).
    /// - `buffer` must have at least [`required_len`] bytes.
    /// - `buffer` must be fully zeroed if this is the first instance, future instances can use the same
    ///   buffer.
    /// - `data_len` must be a multiple of 8.
    pub unsafe fn new_from_ptr(
        buffer: *mut u8,
        data_len: usize,
        data_slots_ring_len: usize,
    ) -> Self {
        // Make sure buffer is 8-byte aligned and data_len is a multiple of 8.
        assert_eq!(buffer.addr() & 7, 0);
        assert_eq!(data_len & 7, 0);
        assert!(data_slots_ring_len > 1);

        unsafe {
            let writer_mutex_au32 = atomic_u32_from_u8_ptr(buffer);
            let current_data_slot_idx = atomic_u32_from_u8_ptr(buffer.add(4));

            Self {
                data_len,
                buffer,
                writer_mutex: Mutex::new(writer_mutex_au32),
                current_data_slot_idx,
                data_slots_ring_len,
            }
        }
    }

    /// Start a new reader, returning a guard that prevents any writers from mutating the slot
    /// the reader is pointing to.
    ///
    /// This could lead to a deadlock if a read handle is held, and you try to start a writer.
    pub fn start_read(&self) -> DataHandle<'_> {
        loop {
            // Load the current index with an `Acquire` so we load all the writers writes.
            let current_idx = self.current_data_slot_idx.load(Ordering::Acquire) as usize;

            let data_slot_ptr = self.data_slot_ptr(current_idx);

            let data_slot_state = unsafe { DataSlotState::from_u8_ptr(data_slot_ptr) };

            // Try to add a reader to the state, continuing if it has been marked as old.
            if !data_slot_state.check_add_reader() {
                std::hint::spin_loop();
                continue;
            }

            // Get the data slice.
            // Safety:
            //     - the ptr is valid as that is a requirement for `RcuRing`
            //     - no writer will be writing to this slot as we have added a read to the slot state.
            let data =
                unsafe { std::slice::from_raw_parts(data_slot_ptr.add(8).cast(), self.data_len) };

            return DataHandle {
                data_slot_state,
                data,
            };
        }
    }

    /// Start a writer to this RCU.
    ///
    /// Only 1 writer can exist at a time, this is enforced in the ring, if you hold a write and try to
    /// start another it will block the thread.
    ///
    /// - `current_op` is a user-defined value, that you can use with `wait_for_all_readers`
    /// - `wait_for_all_readers` is a function that returns a `bool`, it accepts the `op` of the writer
    ///   which wrote to the _previous_ slot (the `current_op` of the last write operation).
    ///
    /// If `wait_for_all_readers` is `true` this will wait for all readers to be updated to the latest data slot,
    /// if it is `false` we will only wait for the next slot that we want to write to be free of readers.
    pub fn start_write(
        &self,
        current_op: u32,
        wait_for_all_readers: impl FnOnce(u32) -> bool,
    ) -> WriteGuard<'_> {
        let _mutex_guard = self.writer_mutex.lock();

        // Get the current reader slot.
        let current_idx = self.current_data_slot_idx.load(Ordering::Acquire) as usize;
        let current_data_slot_ptr = self.data_slot_ptr(current_idx);
        let current_data_slot_state = unsafe { DataSlotState::from_u8_ptr(current_data_slot_ptr) };

        // Get the operation of the last writer.
        let last_op = unsafe { current_data_slot_ptr.add(4).cast::<u32>().read() };

        // Get the data in the current reader.
        let current_data =
            unsafe { std::slice::from_raw_parts(current_data_slot_ptr.add(8), self.data_len) };

        // Wait for the next slot or for all readers to be at the current slot.
        let next_idx = if wait_for_all_readers(last_op) {
            self.wait_for_all_readers(current_idx)
        } else {
            self.wait_for_next_free_slot(current_idx)
        };

        // Get the ptr to the next slot.
        let data_slot_ptr = self.data_slot_ptr(next_idx);

        // Write our current_op to it.
        unsafe { data_slot_ptr.add(4).cast::<u32>().write(current_op) };

        let next_data_slot_state = unsafe { DataSlotState::from_u8_ptr(data_slot_ptr) };

        // Get a mutable reference to the data we can edit.
        let data = unsafe { std::slice::from_raw_parts_mut(data_slot_ptr.add(8), self.data_len) };

        // Update it with the data currently being read.
        data.copy_from_slice(current_data);

        WriteGuard {
            _mutex_guard,
            current_data_slot_state,
            current_data_slot_idx: self.current_data_slot_idx,
            next_data_slot_idx: next_idx as u32,
            next_data_slot_state,
            data,
        }
    }

    /// Waits for the next reader slot to be have no readers.
    fn wait_for_next_free_slot(&self, current_idx: usize) -> usize {
        let next_free_slot = (current_idx + 1) % self.data_slots_ring_len;
        let data_slot_ptr = self.data_slot_ptr(next_free_slot);

        let data_slot_state = unsafe { DataSlotState::from_u8_ptr(data_slot_ptr) };

        data_slot_state.wait_for_readers();

        next_free_slot
    }

    /// Waits for all slots apart from `current_idx` to have no readers.
    ///
    /// This can be used to ensure all readers have updated to the new value.
    pub fn wait_for_all_readers(&self, current_idx: usize) -> usize {
        for i in 1..self.data_slots_ring_len {
            let data_slot_ptr = self.data_slot_ptr((current_idx + i) % self.data_slots_ring_len);

            let data_slot_state = unsafe { DataSlotState::from_u8_ptr(data_slot_ptr) };

            data_slot_state.wait_for_readers();
        }

        (current_idx + 1) % self.data_slots_ring_len
    }

    /// Returns a ptr to the start of the data slot at the given index.
    fn data_slot_ptr(&self, idx: usize) -> *mut u8 {
        let mdata_slot_ptr_offset = HEADER_LEN + data_slot_len(self.data_len) * idx;

        unsafe { self.buffer.add(mdata_slot_ptr_offset) }
    }
}

/// Returns the length of a data slot.
const fn data_slot_len(data_len: usize) -> usize {
    8 + data_len
}

/// Returns the length the byte buffer needs to be for the given parameters.
pub const fn required_len(data_len: usize, data_slot_ring_len: usize) -> usize {
    8 + data_slot_len(data_len) * data_slot_ring_len
}

unsafe fn atomic_u32_from_u8_ptr(ptr: *mut u8) -> &'static AtomicU32 {
    let ptr = ptr.cast::<u32>();

    assert!(ptr.cast::<AtomicU32>().is_aligned());

    unsafe { AtomicU32::from_ptr(ptr) }
}
