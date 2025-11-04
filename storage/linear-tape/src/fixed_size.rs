use std::{io, marker::PhantomData, ops::Range, path::Path};

use crate::{unsafe_tape::UnsafeTape, Advice, Flush, ResizeNeeded};

/// A type that can be inserted into a [`LinearFixedSizeTape`].
pub trait Entry: Sized {
    /// The size of this type on disk.
    const SIZE: usize;

    /// Write the item to the byte slice provided, this slice will be the exact length of [`Self::SIZE`].
    ///
    /// There are no guarantees made on the contents of the bytes before being written to or the byte's alignment.
    fn write(&self, to: &mut [u8]);

    /// Read an item from a byte slice, this slice will be the exact length of [`Self::SIZE`].
    ///
    /// There are no guarantees made on the byte's alignment.
    fn read(from: &[u8]) -> Self;

    /// Write a batch of items to the slice provided, this slice will be the exact length of [`Self::SIZE`] * len.
    ///
    /// There are no guarantees made on the contents of the bytes before being written to or the byte's alignment.
    fn batch_write(from: &[Self], mut to: &mut [u8]) {
        for this in from {
            this.write(&mut to[..Self::SIZE]);
            to = &mut to[Self::SIZE..];
        }
    }
}

/// A reader for a [`LinearFixedSizeTape`].
pub struct LinearTapeReader<'a, E: Entry> {
    pub backing_file: &'a UnsafeTape,
    pub phantom: PhantomData<E>,
    pub len: usize,
}

impl<E: Entry> LinearTapeReader<'_, E> {
    /// Returns the amount of entries in the tape.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Try to get a value from the tape.
    pub fn try_get(&self, i: usize) -> Option<E> {
        if self.len <= i {
            return None;
        }

        unsafe { Some(E::read(&self.backing_file.range(entry_byte_range::<E>(i)))) }
    }
}

/// A writer for a [`LinearFixedSizeTape`].
pub struct LinearTapeAppender<'a, E: Entry> {
    pub backing_file: &'a UnsafeTape,
    pub phantom: PhantomData<E>,
    pub current_used_bytes: usize,
    pub bytes_added: &'a mut usize,
}

impl<E: Entry> LinearTapeAppender<'_, E> {
    /// Try to get a value from the tape.
    pub fn try_get(&self, i: usize) -> Option<E> {
        if self.len() <= i {
            return None;
        }

        unsafe { Some(E::read(&self.backing_file.range(entry_byte_range::<E>(i)))) }
    }

    /// Returns the length of the tape if this transaction is flushed.
    pub fn len(&self) -> usize {
        (self.current_used_bytes + *self.bytes_added) / E::SIZE
    }

    /// Push some entries onto the tape.
    ///
    /// # Errors
    ///
    /// This will only return an error is a resize is needed to fit all entries onto the tape.
    /// If this happens none of the entries will have been written, but previous writes can still be
    /// flushed by flushing the transaction.
    pub fn push_entries(&mut self, entries: &[E]) -> Result<(), ResizeNeeded> {
        if self.backing_file.map_size() / E::SIZE - self.len() < entries.len() {
            return Err(ResizeNeeded);
        }

        let start = self.current_used_bytes + *self.bytes_added;
        let end = start + entries.len() * E::SIZE;

        let mut buf = unsafe { self.backing_file.range_mut(start..end) };

        E::batch_write(entries, &mut buf);

        *self.bytes_added += entries.len() * E::SIZE;

        Ok(())
    }

    /// Cancel any previous additions that haven't been flushed, once this is done you can reuse this
    /// appender to push more entries onto the tape.
    pub fn cancel(&mut self) {
        *self.bytes_added = 0;
    }
}

/// A [`LinearFixedSizeTape`] popper, used to remove items from the top of the tape.
///
/// This is seperated from a [`LinearTapeAppender`] to enforce atomic writes, popping and appending
/// cannot be combined in an atomic way.
pub struct LinearTapePopper<'a, E: Entry> {
    backing_file: &'a mut UnsafeTape,
    phantom: PhantomData<E>,
    entries_popped: usize,
}

impl<E: Entry> LinearTapePopper<'_, E> {
    /// Pop some entries from the tape.
    pub fn pop_entries(&mut self, amt: usize) {
        self.entries_popped += amt;
    }

    /// Cancel this change, the tape will be restored to where it was before this operation.
    pub fn cancel(&mut self) {
        self.entries_popped = 0;
    }
}

fn entry_byte_range<E: Entry>(i: usize) -> Range<usize> {
    let start = i * E::SIZE;
    let end = start + E::SIZE;

    start..end
}
