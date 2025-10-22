mod blob_tape;

pub use blob_tape::*;

use memmap2::{Advice, MmapMut, MmapOptions, MmapRaw, RemapOptions};
use std::cmp::max;
use std::fs::{File, OpenOptions};
use std::io::{empty, Read, Write};
use std::marker::PhantomData;
use std::ops::{Deref, Index, Range};
use std::path::Path;
use std::{fs, io, slice};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A type that can be inserted into a [`LinearTape`].
pub trait Entry {
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
}

/// The header of a [`LinearTape`]
struct DatabaseHeader {
    /// The [`LinearTape`] version.
    version: u32,
    /// The number of entries in the [`LinearTape`].
    entries: usize,
    /// The size of a single [`Entry`].
    entry_len: usize,
}

impl DatabaseHeader {
    /// The size of the [`DatabaseHeader`] on disk.
    const SIZE: usize = 20;

    /// Read a [`DatabaseHeader`] from a byte slice.
    fn read(buf: &[u8]) -> Self {
        DatabaseHeader {
            version: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            entries: usize::from_le_bytes(buf[4..12].try_into().unwrap()),
            entry_len: usize::from_le_bytes(buf[12..20].try_into().unwrap()),
        }
    }

    /// Convert this [`DatabaseHeader`] to bytes
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..4].clone_from_slice(&self.version.to_le_bytes());
        buf[4..12].copy_from_slice(&self.entries.to_le_bytes());
        buf[12..20].copy_from_slice(&self.entry_len.to_le_bytes());
        buf
    }
}

/// The backing memory map file.
struct BackingFile {
    /// The [`File`] that the memory map points to.
    file: File,
    /// The memory map.
    mmap: MmapRaw,
    /// The amount of [`Entry`]s in this [`LinearTape`].
    len: AtomicUsize,
}

impl BackingFile {
    /// Take a range of bytes from the memory map.
    ///
    /// # Safety
    ///
    /// This memory map must be initialised in this range.
    unsafe fn range(&self, range: Range<usize>) -> &[u8] {
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
    unsafe fn range_mut(&self, range: Range<usize>) -> &mut [u8] {
        unsafe {
            let ptr = self.mmap.as_mut_ptr().add(range.start);
            slice::from_raw_parts_mut(ptr, range.len())
        }
    }

    fn capacity(&self, data_size: usize) -> usize {
        capacity(self.file.metadata().unwrap().len(), data_size) - self.len.load(Ordering::Acquire)
    }

    /// Flushes outstanding memory map modifications to disk.
    fn flush(&self, new_len: usize) -> io::Result<()> {
        self.len.store(new_len, Ordering::Release);
        // TODO: this needs to be atomic if we support multiple processes opening the same file.
        unsafe {
            self.range_mut(4..12).copy_from_slice(&new_len.to_le_bytes());
        }
        self.mmap.flush()
    }

    /// Asynchronously flushes outstanding memory map modifications to disk.
    ///
    /// This method initiates flushing modified pages to durable storage, but it will not wait for
    /// the operation to complete before returning.
    fn flush_async(&self, new_len: usize) -> io::Result<()> {
        self.len.store(new_len, Ordering::Release);
        // TODO: this needs to be atomic if we support multiple processes opening the same file.
        unsafe {
            self.range_mut(4..12).copy_from_slice(&new_len.to_le_bytes());
        }
        self.mmap.flush_async()
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
pub struct LinearTape<E: Entry> {
    backing_file: BackingFile,
    phantom: PhantomData<E>,
}

impl<E: Entry> LinearTape<E> {
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
                entry_len: E::SIZE,
            };

            file.write_all(&header.to_bytes())?;
            file.flush()?;
            file.set_len(4096 * 1024 * 1024 * 1024)?;

            header
        } else {
            let mut header = [0u8; DatabaseHeader::SIZE];
            file.read_exact(&mut header)?;
            let header = DatabaseHeader::read(&header);

            if header.entry_len != E::SIZE {
                panic!()
            }

            header
        };

        let mmap =  MmapOptions::new().no_reserve_swap().map_raw(&file)?;

        mmap.advise(Advice::Random)?;

        Ok(LinearTape {
            backing_file: BackingFile {
                file,
                mmap,
                len: header.entries.into(),
            },
            phantom: PhantomData,
        })
    }

    pub fn len(&self) -> usize {
        self.backing_file.len.load(Ordering::Acquire)
    }

    pub fn try_get(&self, i: usize) -> Option<E> {
        if self.len() <= i {
            return None;
        }

        unsafe { Some(E::read(&self.backing_file.range(entry_byte_range::<E>(i)))) }
    }

    pub fn resize(&mut self, new_size_bytes: u64) -> io::Result<()> {
        self.backing_file.mmap.flush()?;

        self.backing_file.file.set_len(new_size_bytes)?;

        let map = MmapOptions::new().map_raw(&self.backing_file.file)?;
        map.advise(Advice::Random)?;

        self.backing_file.mmap = map;

        Ok(())
    }

    pub fn appender(&self) -> LinearTapeAppender<'_, E> {
        LinearTapeAppender {
            backing_file: &self.backing_file,
            phantom: PhantomData,
            entries_added: 0,
        }
    }

    pub fn popper(&mut self) -> LinearTapePopper<'_, E> {
        LinearTapePopper {
            backing_file: &mut self.backing_file,
            phantom: PhantomData,
            entries_popped: 0,
        }
    }
}

/// A writer for a [`LinearTape`].
///
/// # Atomicity
///
/// Make sure to check each function for atomicity as not all functions are atomic.
pub struct LinearTapeAppender<'a, E: Entry> {
    backing_file: &'a BackingFile,
    phantom: PhantomData<E>,
    entries_added: usize,
}

impl<E: Entry> Drop for LinearTapeAppender<'_, E> {
    fn drop(&mut self) {
        if self.entries_added != 0 {
            drop(self.flush_async());
        }
    }
}

impl<E: Entry> LinearTapeAppender<'_, E> {
    /// Push some entries onto the tape.
    ///
    /// # Atomicity
    ///
    /// This function is atomic, it can be called multiple times, and it will still be atomic.
    /// However, if it is paired with [`LinearTapeWriter::pop_entries`] then it will no longer be atomic.
    pub fn push_entries(&mut self, entries: &[E]) -> Result<(), ResizeNeeded> {
        if self.backing_file.capacity(E::SIZE) < entries.len() {
            return Err(ResizeNeeded);
        }

        let start = (self.backing_file.len.load(Ordering::Acquire) + self.entries_added) * E::SIZE + DatabaseHeader::SIZE;
        let end = start + entries.len() * E::SIZE;

        let mut buf = unsafe { self.backing_file.range_mut(start..end) };

        for e in entries {
            e.write(&mut buf[..E::SIZE]);
            buf = &mut buf[E::SIZE..];
        }

        self.entries_added += entries.len();

        Ok(())
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

/// A [`LinearTape`] popper, used to remove items from the top of the tape.
///
/// This is seperated from a [`LinearTapeAppender`] to enforce atomic writes, popping and appending
/// cannot be combined in an atomic way.
pub struct LinearTapePopper<'a, E: Entry> {
    backing_file: &'a mut BackingFile,
    phantom: PhantomData<E>,
    entries_popped: usize,
}

impl<E: Entry> Drop for LinearTapePopper<'_, E> {
    fn drop(&mut self) {
        if self.entries_popped != 0 {
            drop(self.flush_async());
        }
    }
}

impl<E: Entry> LinearTapePopper<'_, E> {
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

#[derive(Debug)]
pub struct ResizeNeeded;

fn capacity(database_size: u64, data_size: usize) -> usize {
    (database_size as usize - DatabaseHeader::SIZE) / data_size
}

fn entry_byte_range<E: Entry>(i: usize) -> Range<usize> {
    let start = i * E::SIZE + DatabaseHeader::SIZE;
    let end = start + E::SIZE;

    start..end
}

#[test]
fn t() {
    #[derive(Debug)]
    struct H([u8; 32]);

    impl Entry for H {
        const SIZE: usize = 32;

        fn write(&self, to: &mut [u8]) {
            to.copy_from_slice(&self.0)
        }

        fn read(from: &[u8]) -> Self {
            Self(from.try_into().unwrap())
        }
    }

    let mut tape = unsafe { LinearTape::open("test.tape") }.unwrap();

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
