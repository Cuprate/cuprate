use std::{marker::PhantomData, ops::Range, io, path::Path};

use crate::{Advice, Flush, ResizeNeeded, unsafe_tape::UnsafeTape};

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

/// A linear tape database.
///
/// A linear tape stores fixed sized entries in an array like structure.
/// It supports pushing and popping values to and from the top, and random lookup
/// by the indexes of entries.
pub struct LinearFixedSizeTape<E: Entry> {
    backing_file: UnsafeTape,
    phantom: PhantomData<E>,
}

impl<E: Entry> LinearFixedSizeTape<E> {
    /// Open a [`LinearFixedSizeTape`], creating a new database if it does not exist.
    ///
    /// # Safety
    ///
    /// See the [crate docs](crate).
    pub unsafe fn open<P: AsRef<Path>>(path: P, advice: Advice, initial_map_size: u64) -> io::Result<Self> {
        let backing_file = unsafe { UnsafeTape::open(path, advice, initial_map_size)? };

        if backing_file.used_bytes() % E::SIZE != 0 {
            return Err(io::Error::other("Database has invalid size, not multiple of entry size"));
        }

        Ok(LinearFixedSizeTape {
            backing_file,
            phantom: PhantomData,
        })
    }

    /// Returns the size of the memory map.
    pub fn map_size(&self) -> usize {
        self.backing_file.map_size()
    }

    /// Resize the tape.
    ///
    /// This will clamp the size to the size of the underlying file, so you can use `0` as the new size
    /// if another process resizes the file to accept that new size.
    pub fn resize(&mut self, new_size_bytes: u64) -> io::Result<()> {
        self.backing_file.resize(new_size_bytes)
    }

    /// Start a tape reader.
    pub fn reader(&self) -> Result<LinearTapeReader<E>, ResizeNeeded> {
        let used_bytes  = self.backing_file.used_bytes();

        if used_bytes > self.backing_file.usable_map_size() {
            return Err(ResizeNeeded)
        };

        Ok(
            LinearTapeReader {
                backing_file: &self.backing_file,
                phantom: Default::default(),
                len: used_bytes / E::SIZE,
            }
        )
    }

    /// Open a new [`LinearTapeAppender`] to push items to the end of the tape.
    ///
    /// # Safety
    ///
    /// You must ensure only 1 appender is active at a time, see the [crate docs](crate) for more.
    pub unsafe fn appender(&self) -> Result<LinearTapeAppender<'_, E>, ResizeNeeded> {
        if self.backing_file.need_resize() {
            return Err(ResizeNeeded)
        }

        // Because only 1 appender can be active at a time we don't have to worry about the file growing
        // beyond the map from here.

        Ok(LinearTapeAppender {
            backing_file: &self.backing_file,
            phantom: PhantomData,
            entries_added: 0,
        })
    }

    /// Open a new [`LinearTapePopper`] to remove items from the end of the tape.
    pub fn popper(&mut self) -> LinearTapePopper<'_, E> {
        // We don't need to check if we need a resize as `LinearTapePopper` doesn't access any bytes
        // apart from the first 8.

        LinearTapePopper {
            backing_file: &mut self.backing_file,
            phantom: PhantomData,
            entries_popped: 0,
        }
    }
}

/// A reader for a [`LinearFixedSizeTape`].
pub struct LinearTapeReader<'a, E: Entry> {
    backing_file: &'a UnsafeTape,
    phantom: PhantomData<E>,
    len: usize,
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
    backing_file: &'a UnsafeTape,
    phantom: PhantomData<E>,
    entries_added: usize,
}

impl<E: Entry> LinearTapeAppender<'_, E> {
    /// Returns the length of the tape if this transaction is flushed.
    pub fn len(&self) -> usize {
        self.backing_file.used_bytes() / E::SIZE + self.entries_added
    }

    /// Push some entries onto the tape.
    ///
    /// # Errors
    ///
    /// This will only return an error is a resize is needed to fit all entries onto the tape.
    /// If this happens none of the entries will have been written, but previous writes can still be
    /// flushed by flushing the transaction.
    pub fn push_entries(&mut self, entries: &[E]) -> Result<(), ResizeNeeded> {
        if self.backing_file.free_capacity() / E::SIZE < self.entries_added + entries.len() {
            return Err(ResizeNeeded);
        }

        let start = self.backing_file.used_bytes() + self.entries_added * E::SIZE;
        let end = start + entries.len() * E::SIZE;

        let mut buf = unsafe { self.backing_file.range_mut(start..end) };

        E::batch_write(entries, &mut buf);

        self.entries_added += entries.len();

        Ok(())
    }

    /// Flush the transaction to disk.
    pub fn flush(&mut self, mode: Flush) -> io::Result<()> {
        self.backing_file.extend(mode, self.entries_added * E::SIZE)?;
        self.entries_added = 0;

        Ok(())
    }

    /// Cancel any previous additions that haven't been flushed, once this is done you can reuse this
    /// appender to push more entries onto the tape.
    pub fn cancel(&mut self) {
        self.entries_added = 0;
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

    /// Flush the tape to disk, saving any changes made.
    pub fn flush(&mut self, mode: Flush) -> io::Result<()> {
        self.backing_file.remove_bytes(mode, self.entries_popped * E::SIZE)?;
        self.entries_popped = 0;
        Ok(())
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
