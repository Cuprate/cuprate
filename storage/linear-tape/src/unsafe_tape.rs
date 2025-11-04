use memmap2::{MmapOptions, MmapRaw};
use std::cmp::max;
use std::fs::{File, OpenOptions};
use std::io::{Seek, Write};
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{io, slice};

use crate::{Advice, Flush};

#[cfg(target_endian = "big")]
const BE_NOT_SUPPORTED: u8 = panic!();

/// A raw tape database that is used to build the other tape databases.
pub(crate) struct UnsafeTape {
    /// The [`File`] that the memory map points to.
    file: File,
    /// The memory map.
    mmap: MmapRaw,
    /// The [`Advice`] used on the database.
    advice: Advice,
}

impl UnsafeTape {
    /// Open an [`UnsafeTape`].
    ///
    /// # Safety
    ///
    /// This is marked unsafe as modifications to the underlying file can lead to UB.
    /// You must ensure across all processes either there are not more than one [`LinearBlobTape::appender`] and
    /// truncations are done without readers or appenders.
    pub(crate) unsafe fn open<P: AsRef<Path>>(
        path: P,
        advice: Advice,
        initial_map_size: u64,
    ) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.as_ref())?;

        let len = file.metadata()?.len();

        if len < initial_map_size {
            file.set_len(initial_map_size)?;
        }

        let mmap = MmapOptions::new().map_raw(&file)?;
        mmap.advise(advice.to_memmap2_advice())?;

        Ok(UnsafeTape { file, mmap, advice })
    }

    /// Take a slice of bytes from the memory map.
    ///
    /// # Safety
    ///
    /// This memory map must be initialised in this range.
    pub(crate) unsafe fn slice(&self, start: usize, len: usize) -> &[u8] {
        unsafe {
            let ptr = self.mmap.as_ptr().add(start);
            slice::from_raw_parts(ptr, len)
        }
    }

    /// Take a range of bytes from the memory map.
    ///
    /// # Safety
    ///
    /// This memory map must be initialised in this range.
    pub(crate) unsafe fn range(&self, range: Range<usize>) -> &[u8] {
        unsafe {
            let ptr = self.mmap.as_ptr().add(range.start);
            slice::from_raw_parts(ptr, range.len())
        }
    }

    /// Take a mutable range of bytes from the memory map.
    ///
    /// # Safety
    ///
    /// This memory map must be initialised in this range.
    pub(crate) unsafe fn range_mut(&self, range: Range<usize>) -> &mut [u8] {
        unsafe {
            let ptr = self.mmap.as_mut_ptr().add(range.start);
            slice::from_raw_parts_mut(ptr, range.len())
        }
    }

    /// Take a mutable range of bytes from the memory map.
    ///
    /// # Safety
    ///
    /// This memory map must be initialised in this range.
    pub(crate) fn flush_range(&self, offset: usize, len: usize) -> io::Result<()> {
        self.mmap.flush_range(offset, len)
    }

    /// Take a mutable range of bytes from the memory map.
    ///
    /// # Safety
    ///
    /// This memory map must be initialised in this range.
    pub(crate) fn flush_range_async(&self, offset: usize, len: usize) -> io::Result<()> {
        self.mmap.flush_async_range(offset, len)
    }

    /// Returns the total map size, including the header which can't be used for values.
    pub(crate) fn map_size(&self) -> usize {
        self.mmap.len()
    }

    /// Resize the database.
    pub(crate) fn resize(&mut self, new_size_bytes: u64) -> io::Result<()> {
        self.mmap.flush()?;

        let current_len = self.file.metadata()?.len();
        self.file.set_len(max(new_size_bytes, current_len))?;

        let mmap = MmapOptions::new().map_raw(&self.file)?;
        mmap.advise(self.advice.to_memmap2_advice())?;

        self.mmap = mmap;

        Ok(())
    }
}
