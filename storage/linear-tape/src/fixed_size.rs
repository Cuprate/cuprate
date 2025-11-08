use std::{cmp::max, io, marker::PhantomData, ops::Range};

use crate::unsafe_tape::UnsafeTape;

/// A type that can be inserted into a fixed-sized tape.
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

/// A reader for a fixed-sized tape.
pub struct FixedSizedTapeReader<'a, E: Entry> {
    /// The backing tape file.
    pub(crate) backing_file: &'a UnsafeTape,
    /// The amount of fixed-sized objects in the tape.
    pub(crate) len: usize,
    pub(crate) phantom: PhantomData<E>,
}

impl<E: Entry> FixedSizedTapeReader<'_, E> {
    /// Returns the amount of entries in the tape.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Try to get a value from the tape.
    ///
    /// returns [`None`] if the index is out of range.
    pub fn try_get(&self, i: usize) -> Option<E> {
        if self.len <= i {
            return None;
        }

        // Safety: we checked the index is in range above.
        unsafe { Some(E::read(self.backing_file.range(entry_byte_range::<E>(i)))) }
    }
}

/// An appender for a fixed-sized tape.
pub struct FixedSizedTapeAppender<'a, E: Entry> {
    /// The backing tape file that all up-to-date readers see.
    pub(crate) backing_file: &'a UnsafeTape,
    /// A mutable reference to a slot we can put a new instance of a tape in, if it needed a resize.
    pub(crate) resized_backing_file: &'a mut Option<UnsafeTape>,

    /// The minimum amount to resize by.
    pub(crate) min_resize: u64,
    /// The current amount of bytes used in the database, that up-to-date readers will be seeing.
    pub(crate) current_used_bytes: usize,
    /// The amount of bytes that have been added to this tape.
    pub(crate) bytes_added: &'a mut usize,

    pub(crate) phantom: PhantomData<E>,
}

impl<E: Entry> FixedSizedTapeAppender<'_, E> {
    /// Returns the backing tape handle that should be used for all operations in this writer
    fn backing_file(&self) -> &UnsafeTape {
        // If we have a resized tape then we need to use that, otherwise we can use the tape currently
        // in the database object.
        self.resized_backing_file
            .as_ref()
            .unwrap_or(self.backing_file)
    }

    /// Resize the tape to fit adding data of the given length.
    ///
    /// If [`None`] it will resize to the size needed to see all data in the database.
    fn resize_to_fit_extra(&mut self, needed_bytes: Option<usize>) -> io::Result<()> {
        // If we actually need to fit more data, make sure we don't do a resize less than `min_resize`, otherwise just accept
        // whatever the length of the file is.
        let extra_bytes =
            needed_bytes.map_or(0, |needed_bytes| max(needed_bytes as u64, self.min_resize));

        let new_size = (self.current_used_bytes + *self.bytes_added) as u64 + extra_bytes;

        if let Some(resized_backing_file) = self.resized_backing_file.as_mut() {
            resized_backing_file.resize_to_bytes(new_size)?;
            return Ok(());
        }

        *self.resized_backing_file = Some(self.backing_file.resize_to_bytes_copy(new_size)?);

        Ok(())
    }

    /// Try to get a value from the tape.
    ///
    /// returns [`None`] if the index is out of range.
    ///
    /// # Errors
    ///
    /// This will error if a resize attempt failed.
    pub fn try_get(&mut self, i: usize) -> io::Result<Option<E>> {
        if self.len() <= i {
            return Ok(None);
        }

        if self.backing_file().map_size() < self.current_used_bytes + *self.bytes_added {
            self.resize_to_fit_extra(None)?;
        }

        // Safety: we just checked we have enough bytes above.
        unsafe {
            Ok(Some(E::read(
                self.backing_file().range(entry_byte_range::<E>(i)),
            )))
        }
    }

    /// Returns the length of the tape including data written in this transaction.
    pub fn len(&self) -> usize {
        (self.current_used_bytes + *self.bytes_added) / E::SIZE
    }

    /// Push some entries onto the tape.
    ///
    /// # Errors
    ///
    /// This will error if a resize is needed and a resize attempt failed.
    pub fn push_entries(&mut self, entries: &[E]) -> io::Result<()> {
        let bytes_needed = entries.len() * E::SIZE;

        if self.backing_file().map_size()
            < bytes_needed + self.current_used_bytes + *self.bytes_added
        {
            self.resize_to_fit_extra(Some(bytes_needed))?;
        }

        let start = self.current_used_bytes + *self.bytes_added;
        let end = start + bytes_needed;

        // Safety: We take a mutable reference to self and the range this writer can write in is synchronised by the
        // metadata.
        let buf = unsafe { self.backing_file().range_mut(start..end) };

        E::batch_write(entries, buf);

        *self.bytes_added += bytes_needed;

        Ok(())
    }
}

/// A fixed-sized tape popper.
pub struct FixedSizedTapePopper<'a, E: Entry> {
    #[expect(dead_code)]
    /// The backing tape (unused currently, but this type could access data in the future?)
    pub(crate) backing_file: &'a UnsafeTape,
    /// A mutable reference to the new value for the amount of used bytes in this tape.
    pub(crate) current_used_bytes: &'a mut usize,
    pub(crate) phantom: PhantomData<E>,
}

impl<E: Entry> FixedSizedTapePopper<'_, E> {
    /// Pop some entries from the tape.
    ///
    /// # Panics
    ///
    /// This will panic if more entries are popped than are in the tape.
    pub fn pop_entries(&mut self, amt: usize) {
        *self.current_used_bytes = self.current_used_bytes.checked_sub(amt * E::SIZE).unwrap();
    }
}

const fn entry_byte_range<E: Entry>(i: usize) -> Range<usize> {
    let start = i * E::SIZE;
    let end = start + E::SIZE;

    start..end
}
