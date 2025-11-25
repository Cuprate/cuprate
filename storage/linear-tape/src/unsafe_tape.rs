#![expect(clippy::undocumented_unsafe_blocks)]

use std::{
    cmp::max,
    fs::{File, OpenOptions},
    io,
    ops::Range,
    path::Path,
    slice,
};

use memmap2::{MmapOptions, MmapRaw};

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
    pub(crate) unsafe fn open<P: AsRef<Path>>(
        path: P,
        advice: Advice,
        initial_map_size: u64,
    ) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .create(true)
            .open(path.as_ref())?;

        let len = file.metadata()?.len();

        if len < initial_map_size {
            file.set_len(initial_map_size)?;
        }

        let mmap = MmapOptions::new().map_raw(&file)?;
        mmap.advise(advice.to_memmap2_advice())?;

        Ok(Self { file, mmap, advice })
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
    /// This memory map must be initialised in this range, and there must not be multiple references to the same range.
    #[expect(clippy::mut_from_ref)]
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
    pub(crate) fn flush_range(&self, offset: usize, len: usize, mode: Flush) -> io::Result<()> {
        match mode {
            Flush::Sync => self.mmap.flush_range(offset, len),
            Flush::Async => self.mmap.flush_async_range(offset, len),
            Flush::NoSync => Ok(())
        }
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

    /// Resizes the current database to `needed_len`.
    pub(crate) fn resize_to_bytes(&mut self, needed_len: u64) -> io::Result<()> {
        let current_len = self.file.metadata()?.len();
        let new_len = max(current_len, needed_len);

        if new_len != current_len {
            self.file.set_len(max(current_len, needed_len))?;
        }

        let mmap = MmapOptions::new().map_raw(&self.file)?;
        mmap.advise(self.advice.to_memmap2_advice())?;

        self.mmap = mmap;

        Ok(())
    }

    /// Opens a new copy of this database that has been resized to `needed_len`.
    pub(crate) fn resize_to_bytes_copy(&self, needed_len: u64) -> io::Result<Self> {
        let current_len = self.file.metadata()?.len();
        let new_len = max(current_len, needed_len);

        if new_len != current_len {
            self.file.set_len(max(current_len, needed_len))?;
        }

        let mmap = MmapOptions::new().map_raw(&self.file)?;
        mmap.advise(self.advice.to_memmap2_advice())?;

        Ok(Self {
            file: self.file.try_clone()?,
            mmap,
            advice: self.advice,
        })
    }
}
