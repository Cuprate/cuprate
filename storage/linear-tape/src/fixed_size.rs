use std::{cmp::max, io, marker::PhantomData, ops::Deref, cell::RefMut};

use bytemuck::Pod;

use crate::unsafe_tape::UnsafeTape;

/// The full range of values in a fixed-sized tape.
pub struct FixedSizeTapeSlice<'a, T> {
    pub(crate) slice: &'a [T],
}

impl<T> Deref for FixedSizeTapeSlice<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.slice
    }
}


/// An appender for a fixed-sized tape.
pub struct FixedSizedTapeAppender<'a, P: Pod> {
    /// The backing tape file that all up-to-date readers see.
    pub(crate) backing_file: &'a UnsafeTape,
    /// A mutable reference to a slot we can put a new instance of a tape in, if it needed a resize.
    pub(crate) resized_backing_file: RefMut<'a, Option<UnsafeTape>>,

    /// The minimum amount to resize by.
    pub(crate) min_resize: u64,
    /// The current amount of bytes used in the database, that up-to-date readers will be seeing.
    pub(crate) current_used_bytes: usize,
    /// The amount of bytes that have been added to this tape.
    pub(crate) bytes_added: RefMut<'a, usize>,

    pub(crate) phantom: PhantomData<P>,
}

impl<P: Pod> FixedSizedTapeAppender<'_, P> {
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

    pub fn reader_slice(&mut self) -> io::Result<FixedSizeTapeSlice<'_, P>> {
        if self.backing_file().map_size() < self.current_used_bytes + *self.bytes_added {
            self.resize_to_fit_extra(None)?;
        }

        let bytes = unsafe { self.backing_file().slice(0, self.current_used_bytes + *self.bytes_added) };

        Ok(FixedSizeTapeSlice {
            slice: bytemuck::cast_slice(bytes),
        })
    }

    /// Returns the length of the tape including data written in this transaction.
    pub fn len(&self) -> usize {
        (self.current_used_bytes + *self.bytes_added) / size_of::<P>()
    }

    /// Get a mutable slice to a region above the tape that can be written to.
    /// The slice will be indexed from the top of the region that the readers are reading. So `0` will
    /// be the first slot that you can write to.
    ///
    /// The current values in the slice will be meaningless unless already written to.
    ///
    /// # Note
    /// 
    /// It is important to remember that the amount of entries in the tape will be increased by the
    /// capacity no matter if you write to each entry in the returned slice or not.
    /// 
    /// # Errors
    ///
    /// This will error if a resize is needed and a resize attempt failed.
    pub fn slice_to_write(&mut self, capacity: usize) -> io::Result<&mut [P]> {
        let bytes_needed = capacity * size_of::<P>();

        if self.backing_file().map_size()
            < bytes_needed + self.current_used_bytes + *self.bytes_added
        {
            self.resize_to_fit_extra(Some(bytes_needed))?;
        }

        let start = self.current_used_bytes + *self.bytes_added;
        let end = start + bytes_needed;

        *self.bytes_added += bytes_needed;

        // Safety: We take a mutable reference to self and the range this writer can write in is synchronised by the
        // metadata.
        let buf = unsafe { self.backing_file().range_mut(start..end) };
        
        Ok(bytemuck::cast_slice_mut(buf))
    }
}

/// A fixed-sized tape popper.
pub struct FixedSizedTapePopper<'a, P: Pod> {
    #[expect(dead_code)]
    /// The backing tape (unused currently, but this type could access data in the future?)
    pub(crate) backing_file: &'a UnsafeTape,
    /// A mutable reference to the new value for the amount of used bytes in this tape.
    pub(crate) current_used_bytes: &'a mut usize,
    pub(crate) phantom: PhantomData<P>,
}

impl<P: Pod> FixedSizedTapePopper<'_, P> {
    pub fn pop_last(&mut self) -> Option<(usize, &P)> {
        if *self.current_used_bytes == 0 {
            return None;
        }

        let start_idx = *self.current_used_bytes - size_of::<P>();

        // Safety: we are taking a non-mutable reference and we know this is in range.
        let last = unsafe { self.backing_file.slice(start_idx, size_of::<P>()) };

        *self.current_used_bytes = start_idx;

        Some((start_idx / size_of::<P>(), bytemuck::from_bytes(last)))
    }

    /// Set the length of the tape to the given length.
    ///
    /// # Panics
    ///
    /// This will panic if you give a value higher than the last recorded one.
    /// So once you call this function with a value, if you call it again it must be with a value
    /// less than or equal to.
    pub fn set_new_len(&mut self, new_len: usize) {
        let new_len = new_len * size_of::<P>();

        assert!(new_len <= *self.current_used_bytes);
        *self.current_used_bytes = new_len;
    }

    /// Pop some entries from the tape.
    ///
    /// # Panics
    ///
    /// This will panic if more entries are popped than are in the tape.
    pub fn pop_entries(&mut self, amt: usize) {
        *self.current_used_bytes = self.current_used_bytes.checked_sub(amt * size_of::<P>()).unwrap();
    }
}
