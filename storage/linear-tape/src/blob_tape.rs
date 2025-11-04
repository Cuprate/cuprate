use std::{path::Path, io};
use std::ops::Range;
use crate::{Advice, Flush, ResizeNeeded};

/// A trait for a type that can b turned into a blob of data.
pub trait Blob {
    /// The length of the bytes.
    fn len(&self) -> usize;

    /// Writes self to `buf`.
    /// 
    /// `buf` will have a length of exactly [`Self::len`].
    fn write(&self, buf: &mut [u8]);
}

impl Blob for &'_ [u8] {
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }
    
    fn write(&self, buf: &mut [u8]) {
        buf.copy_from_slice(self)
    }
}


/// A writer for a [`LinearBlobTape`].
pub struct LinearBlobTapeAppender<'a> {
    pub backing_file: &'a crate::UnsafeTape,
    pub current_used_bytes: usize,
    pub bytes_added: &'a mut usize,
}

impl LinearBlobTapeAppender<'_> {
    pub fn try_get_range(&self, range: Range<usize>) -> Option<&[u8]> {
        if self.current_used_bytes + *self.bytes_added < range.start || self.current_used_bytes + *self.bytes_added < range.end {
            return None;
        }

        unsafe { Some(&self.backing_file.range(range)) }
    }
    
    /// Push some bytes onto the tape.
    /// 
    /// On success this returns the index of the first added byte.
    ///
    /// # Errors 
    ///
    /// On any error it is guaranteed no changes have been made to the database. If the database needs
    /// a resize and error will be returned, if this happens you can flush the current
    /// appender to save any bytes you pushed before and resize with [`LinearBlobTape::resize`].
    pub fn push_bytes(&mut self, blob: &impl Blob) -> Result<usize, ResizeNeeded> {
        if self.backing_file.map_size() - self.current_used_bytes < *self.bytes_added + blob.len() {
            return Err(ResizeNeeded);
        }

        let start = self.current_used_bytes + *self.bytes_added;
        let end = start + blob.len();

        let mut buf = unsafe { self.backing_file.range_mut(start..end) };

        blob.write(&mut buf);

        *self.bytes_added += blob.len();

        Ok(start)
    }
    
    /// Cancel any current changes.
    pub fn cancel(&mut self) {
        *self.bytes_added = 0;
    }
}

pub struct LinearBlobTapeReader<'a> {
    pub backing_file: &'a crate::UnsafeTape,
    pub used_bytes: usize,
}

impl LinearBlobTapeReader<'_> {
    pub fn len(&self) -> usize {
        self.used_bytes
    }
    
    /// Try to get a slice from the database.
    ///
    /// Returns [`None`] if start_idx + len > [`Self::len`]
    pub fn try_get_range(&self, range: Range<usize>) -> Option<&[u8]> {
        if self.used_bytes < range.start || self.used_bytes < range.end {
            return None;
        }

        unsafe { Some(&self.backing_file.range(range)) }
    }


    /// Try to get a slice from the database.
    ///
    /// Returns [`None`] if start_idx + len > [`Self::len`]
    pub fn try_get_slice(&self, start_idx: usize, len: usize) -> Option<&[u8]> {
        if self.used_bytes < start_idx + len {
            return None;
        }

        unsafe { Some(&self.backing_file.slice(start_idx, len)) }
    }
}
