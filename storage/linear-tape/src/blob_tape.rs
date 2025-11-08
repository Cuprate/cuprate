use std::{cmp::max, io, ops::Range};

use crate::unsafe_tape::UnsafeTape;

/// A trait for a type that can be turned into a blob of data.
pub trait Blob {
    /// The length of the bytes.
    fn len(&self) -> usize;

    /// Writes self to `buf`.
    ///
    /// `buf` will have a length of exactly [`Self::len`].
    fn write(&self, buf: &mut [u8]);
}

impl Blob for [u8] {
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }

    fn write(&self, buf: &mut [u8]) {
        buf.copy_from_slice(self);
    }
}

/// A blob tape appender.
pub struct BlobTapeAppender<'a> {
    /// The current [`UnsafeTape`] which is stored in the database object and may be being used
    /// by readers.
    pub(crate) backing_file: &'a UnsafeTape,
    /// A mutable reference to a slot we can put a new instance of a tape in, if it needed a resize.
    pub(crate) resized_backing_file: &'a mut Option<UnsafeTape>,
    /// The minimum amount to resize by.
    pub(crate) min_resize: u64,
    /// The current amount of bytes used in the database, that up-to-date readers will be seeing.
    pub(crate) current_used_bytes: usize,
    /// The amount of bytes that have been added to this tape.
    pub(crate) bytes_added: &'a mut usize,
}

impl BlobTapeAppender<'_> {
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

        // If we have already created a new handle just resize that - we are the only place with a handle to it.
        if let Some(resized_backing_file) = self.resized_backing_file.as_mut() {
            resized_backing_file.resize_to_bytes(new_size)?;
            return Ok(());
        }

        // Otherwise we need to make a new handle as other places could be reading from the handle.
        *self.resized_backing_file = Some(self.backing_file.resize_to_bytes_copy(new_size)?);

        Ok(())
    }

    /// Attempt to get a range of bytes from the database.
    ///
    /// returns [`None`] if the range covers bytes outside the tape.
    ///
    /// # Errors
    ///
    /// This will error if a resize is needed and a resize attempt failed.
    pub fn try_get_range(&mut self, range: Range<usize>) -> io::Result<Option<&[u8]>> {
        if self.current_used_bytes + *self.bytes_added < range.start
            || self.current_used_bytes + *self.bytes_added < range.end
        {
            return Ok(None);
        }

        if self.backing_file().map_size() < self.current_used_bytes + *self.bytes_added {
            self.resize_to_fit_extra(None)?;
        }

        // Safety: This is synchronised by the metadata, and we just checked the slice is in range above.
        unsafe { Ok(Some(self.backing_file().range(range))) }
    }

    /// Push some bytes onto the tape.
    ///
    /// On success this returns the index of the first added byte.
    ///
    /// # Errors
    ///
    /// This will error if a resize is needed and the resize fails.
    pub fn push_bytes<B: Blob + ?Sized>(&mut self, blob: &B) -> io::Result<usize> {
        let blob_len = blob.len();
        if self.backing_file().map_size() < self.current_used_bytes + *self.bytes_added + blob_len {
            self.resize_to_fit_extra(Some(blob_len))?;
        }

        let start = self.current_used_bytes + *self.bytes_added;
        let end = start + blob_len;

        // Safety: we just checked this slice is in range.
        let buf = unsafe { self.backing_file().range_mut(start..end) };

        blob.write(buf);

        *self.bytes_added += blob_len;

        Ok(start)
    }

    /// Cancel any current changes to this specific tape.
    pub fn cancel(&mut self) {
        *self.bytes_added = 0;
    }
}

/// A popper for a blob tape database.
pub struct BlobTapePopper<'a> {
    #[expect(dead_code)]
    /// The backing tape (unused currently, but this type could access data in the future?)
    pub(crate) backing_file: &'a UnsafeTape,
    /// A mutable reference to the new value for the amount of used bytes in this tape.
    pub(crate) current_used_bytes: &'a mut usize,
}

impl BlobTapePopper<'_> {
    /// Set the length of the tape to the given length.
    ///
    /// # Panics
    ///
    /// This will panic if you give a value higher than the last recorded one.
    /// So once you call this function with a value, if you call it again it must be with a value
    /// less than or equal to.
    pub fn set_new_len(&mut self, new_len: usize) {
        assert!(new_len <= *self.current_used_bytes);
        *self.current_used_bytes = new_len;
    }
}

/// A reader for a blob tape.
pub struct BlobTapeReader<'a> {
    /// The backing tape file.
    pub(crate) backing_file: &'a UnsafeTape,
    /// The amount of bytes we are allowed to read.
    pub(crate) used_bytes: usize,
}

impl BlobTapeReader<'_> {
    /// The current length of the tape, from this readers' perspective.
    pub fn len(&self) -> usize {
        self.used_bytes
    }

    /// Try to get a slice from the database.
    ///
    /// Returns [`None`] if the range covers more bytes than we can read.
    pub fn try_get_range(&self, range: Range<usize>) -> Option<&[u8]> {
        if self.used_bytes < range.start || self.used_bytes < range.end {
            return None;
        }

        // Safety: This is synchronised by the metadata, and we just checked the slice is in range above.
        unsafe { Some(self.backing_file.range(range)) }
    }

    /// Try to get a slice from the database.
    ///
    /// Returns [`None`] if  [`Self::len`] < `start_idx` + `len`
    pub fn try_get_slice(&self, start_idx: usize, len: usize) -> Option<&[u8]> {
        if self.used_bytes < start_idx + len {
            return None;
        }

        // Safety: This is synchronised by the metadata, and we just checked the slice is in range above.
        unsafe { Some(self.backing_file.slice(start_idx, len)) }
    }
}
