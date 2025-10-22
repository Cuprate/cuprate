use memmap2::{Advice, MmapMut, MmapOptions, RemapOptions};
use std::cmp::max;
use std::fs::{File, OpenOptions};
use std::io::{empty, Read, Write};
use std::marker::PhantomData;
use std::ops::{Deref, Index, Range};
use std::path::Path;
use std::{fs, io};
use std::sync::atomic::Ordering;
use crate::capacity;

pub trait Blob {
    fn len(&self) -> usize;

    fn write(&self, buf: &mut [u8]);
}

/// The header of a [`crate::LinearTape`]
struct DatabaseHeader {
    /// The [`crate::LinearTape`] version.
    version: u32,
    /// The number of entries in the [`crate::LinearTape`].
    entries: usize,
}

impl DatabaseHeader {
    /// The size of the [`crate::DatabaseHeader`] on disk.
    const SIZE: usize = 20;

    /// Read a [`crate::DatabaseHeader`] from a byte slice.
    fn read(buf: &[u8]) -> Self {
        DatabaseHeader {
            version: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            entries: usize::from_le_bytes(buf[4..12].try_into().unwrap()),
        }
    }

    /// Convert this [`crate::DatabaseHeader`] to bytes
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..4].clone_from_slice(&self.version.to_le_bytes());
        buf[4..12].copy_from_slice(&self.entries.to_le_bytes());
        buf
    }
}

/// A linear tape database.
///
/// A linear tape stores fixed sized entries in an array like structure.
/// It supports pushing and popping values to and from the top, and random lookup
/// by the indexes of entries.
///
/// # Atomicity
///
/// Not all operations are atomic, more details on this can be seen on the write functions.
pub struct LinearBlobTape {
    backing_file: crate::BackingFile,
}

impl LinearBlobTape {
    pub unsafe fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.as_ref())?;

        let new = file.metadata()?.len() == 0;

        let header = if new {
            let header = DatabaseHeader {
                version: 1,
                entries: 0,
            };

            file.write_all(&header.to_bytes())?;
            file.flush()?;

            file.set_len(4096 * 1024 * 1024 * 1024)?;
            header
        } else {
            let mut header = [0u8; DatabaseHeader::SIZE];
            file.read_exact(&mut header)?;
            let header = DatabaseHeader::read(&header);

            header
        };

        let mmap =  MmapOptions::new().no_reserve_swap().map_raw(&file)? ;
        mmap.advise(Advice::Sequential)?;


        Ok(LinearBlobTape {
            backing_file: crate::BackingFile {
                file,
                mmap,
                len: header.entries.into(),
            },
        })
    }

    pub fn len(&self) -> usize {
        self.backing_file.len.load(Ordering::Acquire)
    }

    pub fn try_get_range(&self, range: Range<usize>) -> Option<&[u8]> {
        if self.len() <= range.start || self.len() <= range.end {
            return None;
        }

        unsafe { Some(&self.backing_file.range(range)) }
    }

    pub fn resize(&mut self, new_size_bytes: u64) -> io::Result<()> {
        self.backing_file.mmap.flush()?;

        self.backing_file.file.set_len(new_size_bytes)?;

        let map =  MmapOptions::new().map_raw(&self.backing_file.file)? ;
        map.advise(Advice::Sequential)?;

        self.backing_file.mmap = map;

        Ok(())
    }

    pub fn appender(&self) -> LinearBlobTapeAppender<'_> {
        LinearBlobTapeAppender {
            backing_file: & self.backing_file,
            entries_added: 0,
        }
    }

    pub fn popper(&mut self) -> LinearBlobTapePopper<'_> {
        LinearBlobTapePopper {
            backing_file: &mut self.backing_file,
            entries_popped: 0,
        }
    }
}



/// A writer for a [`crate::LinearTape`].
///
/// # Atomicity
///
/// Make sure to check each function for atomicity as not all functions are atomic.
pub struct LinearBlobTapeAppender<'a> {
    backing_file: &'a crate::BackingFile,
    entries_added: usize,
}

impl Drop for LinearBlobTapeAppender<'_> {
    fn drop(&mut self) {
        if self.entries_added != 0 {
            drop(self.flush_async());
        }
    }
}

impl LinearBlobTapeAppender<'_> {
    /// Push some entries onto the tape.
    ///
    /// # Atomicity
    ///
    /// This function is atomic, it can be called multiple times, and it will still be atomic.
    /// However, if it is paired with [`LinearTapeWriter::pop_entries`] then it will no longer be atomic.
    pub fn push_entry(&mut self, blob: &impl Blob) -> Result<usize, crate::ResizeNeeded> {
        if self.backing_file.capacity(1) < blob.len() {
            return Err(crate::ResizeNeeded);
        }

        let start = self.backing_file.len.load(Ordering::Acquire) + self.entries_added + crate::DatabaseHeader::SIZE;
        let end = start + blob.len();

        let mut buf = unsafe { self.backing_file.range_mut(start..end) };

        blob.write(&mut buf);

        self.entries_added += blob.len();

        Ok(start)
    }

    pub fn len(&self) -> usize {
        self.backing_file.len.load(Ordering::Acquire)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.backing_file.flush(self.backing_file.len.load(Ordering::Acquire) + self.entries_added)?;
        self.entries_added = 0;

        Ok(())
    }

    pub fn flush_async(&mut self) -> io::Result<()> {
        self.backing_file.flush_async(self.backing_file.len.load(Ordering::Acquire) + self.entries_added)?;
        self.entries_added = 0;

        Ok(())
    }

    pub fn cancel(&mut self) {
        self.entries_added = 0;
    }
}

/// A [`crate::LinearTape`] popper, used to remove items from the top of the tape.
///
/// This is seperated from a [`crate::LinearTapeAppender`] to enforce atomic writes, popping and appending
/// cannot be combined in an atomic way.
pub struct LinearBlobTapePopper<'a> {
    backing_file: &'a mut crate::BackingFile,
    entries_popped: usize,
}

impl Drop for LinearBlobTapePopper<'_> {
    fn drop(&mut self) {
        if self.entries_popped != 0 {
            drop(self.flush_async());
        }
    }
}

impl LinearBlobTapePopper<'_> {
    /// Pop some entries from the tape.
    pub fn pop_entries(&mut self, amt: usize) {
        self.entries_popped += amt;
    }

    /// Flush the tape to disk, saving any changes made.
    pub fn flush(&mut self) -> io::Result<()> {
        self.backing_file.flush(self.backing_file.len.load(Ordering::Acquire) - self.entries_popped)?;
        self.entries_popped = 0;
        Ok(())
    }

    /// Asynchronously flushes outstanding memory map modifications to disk.
    ///
    /// This method initiates flushing modified pages to durable storage, but it will not wait for
    /// the operation to complete before returning
    pub fn flush_async(&mut self) -> io::Result<()> {
        self.backing_file.flush_async(self.backing_file.len.load(Ordering::Acquire) - self.entries_popped)?;
        self.entries_popped = 0;
        Ok(())
    }

    /// Cancel this change, the tape will be restored to where it was before this operation.
    pub fn cancel(&mut self) {
        self.entries_popped = 0;
    }
}

#[test]
fn t() {
    #[derive(Debug)]
    struct H([u8; 32]);

    impl crate::Entry for H {
        const SIZE: usize = 32;

        fn write(&self, to: &mut [u8]) {
            to.copy_from_slice(&self.0)
        }

        fn read(from: &[u8]) -> Self {
            Self(from.try_into().unwrap())
        }
    }

    let mut tape = unsafe { crate::LinearTape::open("test.tape") }.unwrap();

    tape.resize(1024 * 1024 * 1024 * 10).unwrap();
    for i in 0_u64..((1024 * 1024 * 1024 * 6) / 32) {
        let i = i.to_le_bytes();
        let mut h = [0; 32];

        h[..8].copy_from_slice(&i[..8]);
        tape.appender().push_entries(&[H(h)]).unwrap();
    }

    let h: H = tape.try_get((1024 * 1024 * 1024 * 6) / 32 - 1).unwrap();
    println!("{:?}", u64::from_le_bytes(h.0[0..8].try_into().unwrap()));
}
