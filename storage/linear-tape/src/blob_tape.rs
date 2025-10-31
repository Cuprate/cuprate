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

/// A linear blob tape database.
///
/// This database stores byte blobs of unspecified size, returning the index they were added at
/// to retrieve them again. It supports truncation and appending bytes.
///
/// This database is multi-reader, single-appender, a truncation must happen without any readers
/// or appender. This is enforced by using Rust's references, a read and appender take an `&` reference
/// whereas truncate takes `&mut`. This is not enforced across multiple instances of [`LinearBlobTape`],
/// this can be done with lock files, etc.
pub struct LinearBlobTape {
    backing_file: crate::UnsafeTape,
}

impl LinearBlobTape {
    /// Open a [`LinearBlobTape`], creating a new database if it is missing.
    ///
    /// # Safety
    ///
    /// This is marked unsafe as modifications to the underlying file can lead to UB.
    /// You must ensure across all processes either there are not more than one [`LinearBlobTape::appender`] and
    /// truncations are done without readers or appenders.
    pub unsafe fn open<P: AsRef<Path>>(path: P, advice: Advice, initial_map_size: u64) -> io::Result<Self> {
        Ok(LinearBlobTape {
            backing_file: unsafe { crate::UnsafeTape::open(path, advice, initial_map_size)? },
        })
    }

    /// Returns the amount of bytes stored in the database.
    pub fn len(&self) -> usize {
        self.backing_file.used_bytes()
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
    
    pub fn reader(&self) -> Result<LinearBlobTapeReader<'_>, ResizeNeeded> {
        let used_bytes  = self.backing_file.used_bytes();

        if used_bytes > self.backing_file.usable_map_size() {
            return Err(ResizeNeeded)
        };
        
        Ok(LinearBlobTapeReader {
            backing_file: &self.backing_file,
            used_bytes
        })
    }

    /// Start an appender, to add some data to the end of the tape.
    ///
    /// When finished you must persist the changes to disk with:
    /// - [`LinearBlobTapeAppender::flush`]
    /// - [`LinearBlobTapeAppender::flush_async`]
    /// 
    /// Otherwise, the changes will not be made.
    /// 
    /// # Safety
    ///
    /// You must not create more than 1 appender at a time, however it is safe to use this while reading.
    pub unsafe fn appender(&self) -> LinearBlobTapeAppender<'_> {
        LinearBlobTapeAppender {
            backing_file: &self.backing_file,
            bytes_added: 0,
        }
    }

    /// Truncate the tape.
    /// 
    /// See [`Self::truncate_async`] for the async version of this function.
    pub fn truncate(&mut self, mode: Flush, new_len: usize) -> io::Result<()> {
        self.backing_file.truncate(mode, new_len)?;
        Ok(())
    }
}

/// A writer for a [`LinearBlobTape`].
pub struct LinearBlobTapeAppender<'a> {
    backing_file: &'a crate::UnsafeTape,
    bytes_added: usize,
}

impl LinearBlobTapeAppender<'_> {
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
        if self.backing_file.free_capacity() < self.bytes_added + blob.len() {
            return Err(ResizeNeeded);
        }

        let start = self.backing_file.used_bytes() + self.bytes_added;
        let end = start + blob.len();

        let mut buf = unsafe { self.backing_file.range_mut(start..end) };

        blob.write(&mut buf);

        self.bytes_added += blob.len();

        Ok(start)
    }
    
    /// Flush the changes to the [`LinearBlobTape`].
    /// 
    /// When this method returns with a non-error result, all outstanding changes to a file-backed 
    /// memory map are guaranteed to be durably stored. The file's metadata (including last modification 
    /// timestamp) may not be updated
    pub fn flush(&mut self, mode: Flush) -> io::Result<()> {
        self.backing_file.extend(mode, self.bytes_added)?;
        self.bytes_added = 0;

        Ok(())
    }
    

    /// Cancel any current changes.
    pub fn cancel(&mut self) {
        self.bytes_added = 0;
    }
}

pub struct LinearBlobTapeReader<'a> {
    backing_file: &'a crate::UnsafeTape,
    used_bytes: usize,
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
