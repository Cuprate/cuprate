use memmap2::{MmapMut, MmapOptions};
use std::cmp::max;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, empty};
use std::marker::PhantomData;
use std::ops::{Index, Range};
use std::path::Path;
use std::{fs, io};

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
struct BackingFile<E: Entry> {
    /// The [`File`] that the memory map points to.
    file: File,
    /// The memory map.
    mmap: MmapMut,
    /// The amount of [`Entry`]s in this [`LinearTape`].
    len: usize,
    /// The number of [`Entry`]s that can fit in this file without a resize.
    free_capacity: usize,
    phantom: PhantomData<E>,
}

impl<E: Entry> BackingFile<E> {
    /// Take a range of bytes from the memory map.
    ///
    /// # Safety
    ///
    /// This memory map must be initialised in this range.
    unsafe fn range(&self, range: Range<usize>) -> &[u8] {
        &self.mmap[range]
    }

    /// Take a mutable range of bytes from the memory map.
    ///
    /// # Safety
    ///
    /// This memory map must be initialised in this range.
    unsafe fn range_mut(&mut self, range: Range<usize>) -> &mut [u8] {
        &mut self.mmap[range]
    }

    /// Flushes outstanding memory map modifications to disk.
    fn flush(&mut self) -> io::Result<()> {
        self.mmap[4..12].copy_from_slice(&self.len.to_le_bytes());
        self.mmap.flush()
    }

    /// Asynchronously flushes outstanding memory map modifications to disk.
    ///
    /// This method initiates flushing modified pages to durable storage, but it will not wait for
    /// the operation to complete before returning.
    fn flush_async(&mut self) -> io::Result<()> {
        self.mmap[4..12].copy_from_slice(&self.len.to_le_bytes());
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
    backing_file: BackingFile<E>,
}

impl<E: Entry> LinearTape<E> {
    pub unsafe fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut new = false;
        let mut file = match OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())
        {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path)?;
                file.set_len(10_000)?;
                new = true;
                file
            }
            Err(e) => return Err(e),
        };

        let header = if new {
            let header = DatabaseHeader {
                version: 1,
                entries: 0,
                entry_len: E::SIZE,
            };

            file.write_all(&header.to_bytes())?;
            file.flush()?;

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

        let free_capacity = capacity::<E>(file.metadata()?.len()) - header.entries;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok(LinearTape {
            backing_file: BackingFile {
                file,
                mmap,
                len: header.entries,
                free_capacity,
                phantom: Default::default(),
            },
        })
    }

    pub fn len(&self) -> usize {
        self.backing_file.len
    }

    pub fn free_capacity(&self) -> usize {
        self.backing_file.free_capacity
    }

    pub fn try_get(&self, i: usize) -> Option<E> {
        if self.backing_file.len <= i {
            return None;
        }

        unsafe { Some(E::read(&self.backing_file.range(entry_byte_range::<E>(i)))) }

    }

    pub fn writer(&mut self) -> LinearTapeWriter<'_, E> {
        LinearTapeWriter {
            backing_file: &mut self.backing_file,
        }
    }
}

impl<E: Entry> Drop for LinearTape<E> {
    fn drop(&mut self) {
        drop(self.backing_file.flush());
    }
}

/// A writer for a [`LinearTape`].
///
///
///
/// # Atomicity
///
/// Make sure to check each function for atomicity as not all functions are atomic.
pub struct LinearTapeWriter<'a, E: Entry> {
    backing_file: &'a mut  BackingFile<E>,
}

impl<E: Entry> LinearTapeWriter<'_, E> {
    fn resize(&mut self, new_size_bytes: u64) -> io::Result<()> {
        self.backing_file.mmap.flush()?;

        self.backing_file.file.set_len(new_size_bytes)?;

        let map = unsafe { MmapOptions::new().map_mut(&self.backing_file.file)? };

        self.backing_file.mmap = map;

        self.backing_file.free_capacity = capacity::<E>(new_size_bytes) - self.backing_file.len;

        Ok(())
    }

    pub fn check_resize(&mut self, new_entries: usize) -> io::Result<()> {
        if self.backing_file.free_capacity < new_entries {
            let old_len = self.backing_file.len * E::SIZE + DatabaseHeader::SIZE;
            let new_len = old_len + new_entries * E::SIZE;

            let new_len = max(new_len, 1024 * 1024 * 1024) as u64;

            self.resize(new_len)?;
        }

        Ok(())
    }

    /// Edit an entry already in the tape.
    ///
    /// # Atomicity
    ///
    /// This function is _NOT_ atomic.
    pub fn edit_entry(&mut self, entry: &E, i: usize) {
        let buf = unsafe { self.backing_file.range_mut(entry_byte_range::<E>(i)) };

        entry.write(buf);
    }

    /// Push some entries onto the tape.
    ///
    /// # Atomicity
    ///
    /// This function is atomic, it can be called multiple times, and it will still be atomic.
    /// However, if it is paired with [`LinearTapeWriter::pop_entries`] then it will no longer be atomic.
    pub fn push_entries(&mut self, entries: &[E]) -> Result<(), ResizeNeeded> {
        if self.backing_file.free_capacity < entries.len() {
            return Err(ResizeNeeded);
        }

        let start = self.backing_file.len * E::SIZE + DatabaseHeader::SIZE;
        let end = start + entries.len() * E::SIZE;

        let mut buf = unsafe { self.backing_file.range_mut(start..end) };

        for e in entries {
            e.write(&mut buf[..E::SIZE]);
            buf = &mut buf[E::SIZE..];
        }

        self.backing_file.len += entries.len();
        self.backing_file.free_capacity -= entries.len();

        Ok(())
    }

    /// Pop some entries from the tape.
    ///
    /// # Atomicity
    ///
    /// This function is atomic, it can be called multiple times, and it will still be atomic.
    /// However, if it is paired with [`LinearTapeWriter::push_entries`] then it will no longer be atomic.
    pub fn pop_entries(&mut self, amt: usize) {
        self.backing_file.len -= amt;
        self.backing_file.free_capacity += amt;
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.backing_file.flush()
    }

    pub fn flush_async(&mut self) -> io::Result<()> {
        self.backing_file.flush_async()
    }
}

pub struct ResizeNeeded;

fn capacity<E: Entry>(size: u64) -> usize {
    (size as usize - DatabaseHeader::SIZE) / E::SIZE
}

fn entry_byte_range<E: Entry>(i: usize) -> Range<usize> {
    let start = i * E::SIZE + DatabaseHeader::SIZE;
    let end = start + E::SIZE;

    start..end
}
