use std::fs::{File, OpenOptions};
use std::ops::Range;
use std::{io, slice};
use std::cmp::max;
use std::io::{Seek, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use memmap2::{ MmapOptions, MmapRaw};

use crate::{Advice, Flush};

#[cfg(target_endian = "big")]
const BE_NOT_SUPPORTED: u8 = panic!();

/// The header of a tape database.
struct DatabaseHeader {
    /// The number of bytes used in the tape, excluding the header.
    bytes_used: usize,
    /// The tape version.
    version: usize,
}

impl DatabaseHeader {
    /// The size of the [`DatabaseHeader`] on disk.
    const SIZE: usize = 16;

    /// Convert this [`DatabaseHeader`] to bytes
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..8].clone_from_slice(&self.bytes_used.to_le_bytes());
        buf[8..16].clone_from_slice(&self.version.to_le_bytes());
        buf
    }
}

/// A raw tape database that is used to build the other tape databases.
pub(crate) struct UnsafeTape {
    /// The [`File`] that the memory map points to.
    file: File,
    /// The memory map.
    mmap: MmapRaw,
    /// The amount of bytes used in the tape.
    ///
    /// # Safety
    ///
    /// This points to the first 8 bytes of [`Self::mmap`], it must not be given out as its lifetime
    /// is tied to the mmap and not to `static`.
    used_bytes: &'static AtomicUsize,
    /// The [`Advice`] used on the database.
    advice: Advice
}

impl UnsafeTape {
    /// Open an [`UnsafeTape`].
    ///
    /// # Safety
    ///
    /// This is marked unsafe as modifications to the underlying file can lead to UB.
    /// You must ensure across all processes either there are not more than one [`LinearBlobTape::appender`] and
    /// truncations are done without readers or appenders.
    pub(crate) unsafe fn open<P: AsRef<Path>>(path: P, advice: Advice, initial_map_size: u64) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.as_ref())?;

        let len = file.metadata()?.len();

        if len == 0 {
            let header = DatabaseHeader {
                version: 1,
                bytes_used: 0,
            };

            file.write_all(&header.to_bytes())?;
            file.flush()?;
        }

        if len < initial_map_size {
            file.set_len(initial_map_size)?;
        }

        let mmap =  MmapOptions::new().map_raw(&file)?;
        mmap.advise(advice.to_memmap2_advice())?;

        let ptr = mmap.as_mut_ptr().cast::<usize>();
        assert!(ptr.cast::<AtomicUsize>().is_aligned());
        let used_bytes = unsafe {
            AtomicUsize::from_ptr(ptr)
        };

        Ok(UnsafeTape {
            file,
            mmap,
            used_bytes,
            advice
        })
    }

    /// Take a slice of bytes from the memory map.
    ///
    /// # Safety
    ///
    /// This memory map must be initialised in this range.
    pub(crate) unsafe fn slice(&self, start: usize, len: usize) -> &[u8] {
        unsafe {
            let ptr = self.mmap.as_ptr().add(start + DatabaseHeader::SIZE);
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
            let ptr = self.mmap.as_ptr().add(range.start + DatabaseHeader::SIZE);
            slice::from_raw_parts(ptr, range.len())
        }
    }

    /// Take a mutable range of bytes from the memory map.
    ///
    /// # Safety
    ///
    /// This memory map must be initialised in this range.
    pub(crate) unsafe fn range_mut(&self, range: Range<usize>) -> &mut [u8] {
        drop(self.mmap.advise_range(memmap2::Advice::PopulateWrite, range.start + DatabaseHeader::SIZE, range.len()));

        unsafe {
            let ptr = self.mmap.as_mut_ptr().add(range.start + DatabaseHeader::SIZE);
            slice::from_raw_parts_mut(ptr, range.len())
        }
    }

    /// Returns if we need to resize the map as the file has grown beyond it by another process.
    pub(crate) fn need_resize(&self) -> bool {
        self.used_bytes() > self.usable_map_size()
    }

    /// Returns the amount of bytes used for values in the map.
    pub(crate) fn used_bytes(&self) -> usize {
        self.used_bytes.load(Ordering::Acquire)
    }

    /// Returns the amount of extra bytes in the tape that can be used for values.
    pub(crate) fn free_capacity(&self) -> usize {
        self.mmap.len() - self.used_bytes.load(Ordering::Acquire) - DatabaseHeader::SIZE
    }

    /// Returns the amount of bytes usable for values in the tape, including bytes already
    /// used for values.
    pub(crate) fn usable_map_size(&self) -> usize {
        self.mmap.len() - DatabaseHeader::SIZE
    }

    /// Returns the total map size, including the header which can't be used for values.
    pub(crate) fn map_size(&self) -> usize {
        self.mmap.len()
    }

    /// Extend the tape a certain number of bytes, this should be done after writing to the bytes.
    pub(crate) fn extend(&self, flush: Flush, extra_bytes: usize) -> io::Result<()> {
        match flush {
            Flush::Sync => self.extend_sync(extra_bytes),
            Flush::Async => self.extend_async(extra_bytes),
            Flush::NoSync => {
                self.used_bytes.fetch_add(extra_bytes, Ordering::Release);
            Ok(())
            },
        }
    }

    fn extend_sync(&self, extra_bytes: usize) -> io::Result<()> {
        let len = self.used_bytes.load(Ordering::Relaxed);
        // Flush the changes
        self.mmap.flush_range(len + DatabaseHeader::SIZE, extra_bytes)?;
        // Now we know the changes are stored, update `used_bytes`.
        // If we do this before the first flush we could leave the data in an invalid state if `used_bytes`
        // gets flushed to disk but the new bytes don't.
        self.used_bytes.store(len + extra_bytes, Ordering::Release);
        // Flush `used_bytes` to disk.
        self.mmap.flush_range(0, 8)
    }

    fn extend_async(&self, extra_bytes: usize) -> io::Result<()> {
        let old_len  = self.used_bytes.fetch_add(extra_bytes, Ordering::Release);

        self.mmap.flush_async_range(old_len + DatabaseHeader::SIZE, extra_bytes)?;
        self.mmap.flush_async_range(0, 8)
    }

    /// Remove a certain number of bytes from the top of the tape.
    pub(crate) fn remove_bytes(&self, flush: Flush, removed_bytes: usize) -> io::Result<()> {
        match flush {
            Flush::Sync => self.remove_bytes_sync(removed_bytes),
            Flush::Async => self.remove_bytes_async(removed_bytes),
            Flush::NoSync => {
                self.used_bytes.fetch_sub(removed_bytes, Ordering::Release);
                Ok(())
            }
        }
    }

    fn remove_bytes_sync(&self, removed_bytes: usize) -> io::Result<()> {
        self.used_bytes.fetch_sub(removed_bytes, Ordering::Release);
        self.mmap.flush_range(0, 8)
    }

    fn remove_bytes_async(&self, removed_bytes: usize) -> io::Result<()> {
        self.used_bytes.fetch_sub(removed_bytes, Ordering::Release);
        self.mmap.flush_async_range(0, 8)
    }

    /// Truncate the tape to a new length.
    pub(crate) fn truncate(&self, flush: Flush, new_len: usize) -> io::Result<()> {
        match flush {
            Flush::Sync => self.truncate_sync(new_len),
            Flush::Async => self.truncate_async(new_len),
            Flush::NoSync => {
                self.used_bytes.store(new_len, Ordering::Release);
                Ok(())
            }
        }
    }

    fn truncate_sync(&self, new_len: usize) -> io::Result<()> {
        self.used_bytes.store(new_len, Ordering::Release);
        self.mmap.flush_range(0, 8)
    }

    fn truncate_async(&self, new_len: usize) -> io::Result<()> {
        self.used_bytes.store(new_len, Ordering::Release);
        self.mmap.flush_async_range(0, 8)
    }

    /// Resize the database.
    pub(crate) fn resize(&mut self, new_size_bytes: u64) -> io::Result<()> {
        self.mmap.flush()?;

        let current_len = self.file.metadata()?.len();
        self.file.set_len(max(new_size_bytes, current_len))?;

        let mmap = MmapOptions::new().map_raw(&self.file)?;
        mmap.advise(self.advice.to_memmap2_advice())?;

        let ptr = mmap.as_mut_ptr().cast::<usize>();
        assert!(ptr.cast::<AtomicUsize>().is_aligned());
        self.used_bytes = unsafe {
            AtomicUsize::from_ptr(ptr)
        };

        self.mmap = mmap;

        Ok(())
    }
}
