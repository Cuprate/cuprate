//! # Linear Tapes
//!
//! ## Overview
//!
//! This crate implements a database system that is specialised for storing data in contiguous tapes,
//! and appending/popping data to/from them.
//!
//! [`LinearTapes`] supports atomically updating multiple tapes at a time with concurrent readers. Write
//! operations have been split into two separate operations: [`Popper`] and [`Appender`], this means you
//! cannot pop and append to different tapes in the same operation.
//!
//! ## Tapes
//!
//! There are two type of tapes, blob and fixed size. You can have a [`LinearTapes`] database made of a mix
//! of them.
//!
//! - fixed sized tapes store fixed sized values and allows direct indexing to get values.
//! - blob tapes store raw bytes, you must store the index the values are at to retrieve them.
//!
//! # Safety
//!
//! All databases are backed by a memory map, which practically can not be worked on in Rust in completely safe code.
//! So you must make sure the file is not edited in a way that is not allowed by this crate.
//!
#![expect(clippy::missing_const_for_fn, clippy::len_without_is_empty)]
#[cfg(test)]
use tempfile as _;

use std::{
    collections::HashMap,
    fs::create_dir_all,
    io,
    path::{Path, PathBuf},
    ptr,
    sync::atomic::{fence, AtomicBool, AtomicPtr, Ordering},
};

mod blob_tape;
mod fixed_size;
mod metadata;
mod unsafe_tape;

pub use blob_tape::{Blob, BlobTapeAppender, BlobTapePopper, BlobTapeReader};
pub use fixed_size::{Entry, FixedSizedTapeAppender, FixedSizedTapePopper, FixedSizedTapeReader};
use metadata::{Metadata, APPEND_OP, POP_OP};
use unsafe_tape::UnsafeTape;

/// The length of the RCU ring for the metadata.
/// More means slow read operations won't slow writes as much, but more slots will need to be checked
/// for no readers when it is required that all readers have seen that latest value. Also, more means
/// more copies of the metadata.
const METADATA_RING_LEN: usize = 8;

/// Advice to give the OS when opening the memory map file.
#[derive(Copy, Clone)]
pub enum Advice {
    /// [`memmap2::Advice::Normal`]
    Normal,
    /// [`memmap2::Advice::Random`]
    Random,
    /// [`memmap2::Advice::Sequential`]
    Sequential,
}

impl Advice {
    const fn to_memmap2_advice(self) -> memmap2::Advice {
        match self {
            Self::Normal => memmap2::Advice::Normal,
            Self::Random => memmap2::Advice::Random,
            Self::Sequential => memmap2::Advice::Sequential,
        }
    }
}

/// The method to use when flushing data.
#[derive(Copy, Clone)]
pub enum Flush {
    /// Function will block until data is persisted.
    Sync,
    /// The flush will be queued, this could leave the database in an invalid state if not all data
    /// is persisted.
    Async,
    /// No explicit synchronisation will be done, the OS has complete control of when data will be persisted.
    NoSync,
}

/// A database resize is needed as the underlying data has grown beyond it.
#[derive(Copy, Clone, Debug)]
pub struct ResizeNeeded;

/// A tape in the database.
pub struct Tape {
    /// The name of the tape, must be unique.
    pub name: &'static str,
    /// The path to use for the tape, if [`None`] will use the default of being under a `tapes` directory
    /// in the metadata directory.
    pub path: Option<PathBuf>,
    /// The advice to open on the database with.
    pub advice: Advice,
}

/// Linear tapes database.
///
/// A linear tape stores data contiguously in the order it is inserted.
pub struct LinearTapes {
    /// The metadata of this database, tracks its state and ensure atomic updates.
    metadata: Metadata,
    /// A map of a tape's name to its index in the list of tapes.
    tapes_to_index: HashMap<&'static str, usize>,

    /// A list of pointers to potentially old tape instances that have not been dropped yet as they could
    /// still have readers.
    old_tapes: Box<[AtomicPtr<UnsafeTape>]>,
    /// An atomic bool for if we have any old tapes that need to be dropped.
    old_tapes_need_flush: AtomicBool,

    /// The list of current tape instances.
    tapes: Box<[AtomicPtr<UnsafeTape>]>,

    /// The minimum step to resize a tape by.
    min_resize: u64,
}

impl Drop for LinearTapes {
    fn drop(&mut self) {
        // Safety: this has been dropped so we know there are no readers pointing to this instance.
        unsafe { self.flush_old_tapes() };
        for tape in &self.tapes {
            // Safety: same as above.
            unsafe {
                let tape = Box::from_raw(tape.load(Ordering::Acquire));

                tape.flush_range()
            }
        }
    }
}

impl LinearTapes {
    /// Open an [`LinearTapes`] database.
    ///
    /// # Safety
    ///
    /// This is marked unsafe as modifications to the underlying file can lead to UB.
    /// You must ensure across all processes that no unsafe accesses are done.
    pub unsafe fn new<P: AsRef<Path>>(
        mut tapes: Vec<Tape>,
        metadata_path: P,
        min_resize: u64,
    ) -> Result<Self, io::Error> {
        create_dir_all(metadata_path.as_ref())?;

        tapes.sort_unstable_by_key(|tape| tape.name);
        // Safety: the requirement is on the caller to uphold the invariants.
        let metadata = unsafe {
            Metadata::open(
                metadata_path.as_ref().join("metadata.tapes"),
                tapes.len(),
                METADATA_RING_LEN,
            )?
        };

        let default_tape_path = metadata_path.as_ref().join("tapes");
        create_dir_all(default_tape_path.as_path())?;

        let tapes_to_index = tapes.iter().enumerate().map(|(i, t)| (t.name, i)).collect();

        let (old_tapes, tapes) = tapes
            .iter()
            .map(|tape| {
                let path = tape.path.as_ref().unwrap_or(&default_tape_path);

                // Safety: the requirement is on the caller to uphold the invariants.
                unsafe {
                    let tape = Box::into_raw(Box::new(UnsafeTape::open(
                        path.join(format!("{}.tape", tape.name)),
                        tape.advice,
                        min_resize,
                    )?));

                    Ok::<_, io::Error>((AtomicPtr::new(ptr::null_mut()), AtomicPtr::new(tape)))
                }
            })
            .collect::<Result<(Vec<_>, Vec<_>), _>>()?;

        Ok(Self {
            tapes_to_index,
            metadata,
            old_tapes: old_tapes.into_boxed_slice(),
            old_tapes_need_flush: AtomicBool::new(false),
            min_resize,
            tapes: tapes.into_boxed_slice(),
        })
    }

    /// Flush old database tapes.
    ///
    /// # Safety
    ///
    /// It must be ensured no other thread is using these tapes.
    unsafe fn flush_old_tapes(&self) {
        for old_tape in &self.old_tapes {
            let ptr = old_tape.swap(ptr::null_mut(), Ordering::Relaxed);
            if ptr.is_null() {
                continue;
            }

            // Make sure the allocations don't get reordered to after the pointer was written.
            fence(Ordering::Acquire);
            // Safety: the above fence means we will see the allocation and its on the caller of this
            // function to ensure there are no users of this tape.
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    }

    /// Start a new database appender.
    ///
    /// Only one write operation can be active at a given time, this will block if another write operation
    /// is active util it is finished. Also after a pop operation the next appender must wait for all readers
    /// to no longer be accessing the bytes in the range popped, so this thread will block waiting for readers
    /// to not be accessing old state if it follows a pop.
    ///
    /// Once you have finished appending data to the tapes your changes must be committed with [`Appender::flush`].
    pub fn appender(&self) -> Appender<'_> {
        let meta_guard = self.metadata.start_write(APPEND_OP);

        // Check if we need to drop an old tape handle.
        // We can use `Relaxed` here as this will be synchronised with the writer mutex in the meta_guard.
        if self
            .old_tapes_need_flush
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            // wait for al readers to be on the current data slot, so we know they are not using the
            // old tape pointer.
            self.metadata
                .wait_for_all_readers(meta_guard.current_data_slot_idx());
            // Safety: we just check above for all readers to be updated.
            unsafe { self.flush_old_tapes() };
        }

        Appender {
            meta_guard,
            tapes: self,
            min_resize: self.min_resize,
            added_bytes: vec![0; self.tapes.len()],
            resized_tapes: (0..self.tapes.len()).map(|_| None).collect(),
        }
    }

    /// Start a new database appender.
    ///
    /// Only one write operation can be active at a given time, this will block if another write operation
    /// is active util it is finished.
    ///
    /// Once you have finished popping data from the tapes your changes must be committed with [`Popper::flush`]
    pub fn popper(&self) -> Popper<'_> {
        let meta_guard = self.metadata.start_write(POP_OP);

        if self
            .old_tapes_need_flush
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            self.metadata
                .wait_for_all_readers(meta_guard.current_data_slot_idx());
            // Safety: same as the appender.
            unsafe { self.flush_old_tapes() };
        }

        Popper {
            meta_guard,
            tapes: self,
        }
    }

    /// Start a database reader.
    ///
    /// # Errors
    ///
    /// This will error if the underlying data in any tape has outgrown our memory map, this can only
    /// happen if another instance of [`LinearTapes`] has extended the file. If all writers and reader
    /// go through the same [`LinearTapes`] this will never error.
    ///
    /// When this errors you will need to resize the tape with: (TODO: reader resizes (will require a specific function that takes the write lock.)
    pub fn reader(&self) -> Result<Reader<'_>, ResizeNeeded> {
        let meta_guard = self.metadata.start_read();

        let loaded_tapes = self
            .tapes
            .iter()
            .map(|tape_ptr| tape_ptr.load(Ordering::Relaxed).cast_const())
            .collect::<Vec<_>>();
        // Make sure the allocations don't get reordered to after the pointers were written.
        fence(Ordering::Acquire);

        for (i, tape) in loaded_tapes.iter().enumerate() {
            // Safety: We are holding a reader guard so this will not be dropped.
            let tape = unsafe { &**tape };

            let bytes_used = meta_guard[i];

            if tape.map_size() < bytes_used {
                return Err(ResizeNeeded);
            }
        }

        Ok(Reader {
            meta_guard,
            loaded_tapes,
            tapes: self,
        })
    }
}

/// An appender for a [`LinearTapes`] database, allows atomically pushing data to multiple tapes.
///
/// To push data to a tape you must open an appender for that tape:
/// - for fixed sized tapes: [`Appender::fixed_sized_tape_appender`]
/// - for blob tapes: [`Appender::blob_tape_appender`]
///
/// Once finished changes must be committed with: [`Appender::flush`]
pub struct Appender<'a> {
    /// The metadata guard for this writer.
    meta_guard: metadata::MetadataWriteGuard<'a>,
    /// The tapes database.
    tapes: &'a LinearTapes,
    /// The minimum step to resize a tape by.
    min_resize: u64,
    /// A vec with the same length as the amount of tapes, representing the amount of bytes that have
    /// been added to each.
    added_bytes: Vec<usize>,
    /// A vec with the same length as the amount of tapes, which holds new tapes handles that have been resized.
    resized_tapes: Vec<Option<UnsafeTape>>,
}

impl Appender<'_> {
    /// Opens a handle to append to a fixed-sized tape.
    pub fn fixed_sized_tape_appender<'a, E: Entry>(
        &'a mut self,
        table_name: &'static str,
    ) -> FixedSizedTapeAppender<'a, E> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        let tape = &self.tapes.tapes[i];

        FixedSizedTapeAppender {
            // Safety: We are holding a write lock and only writers will drop tapes.
            backing_file: unsafe { &*tape.load(Ordering::Acquire) },
            resized_backing_file: &mut self.resized_tapes[i],
            min_resize: self.min_resize,
            phantom: Default::default(),
            current_used_bytes: self.meta_guard.tables_len_mut()[i],
            bytes_added: &mut self.added_bytes[i],
        }
    }

    /// Opens a handle to append to a blob tape.
    pub fn blob_tape_appender<'a>(&'a mut self, table_name: &'static str) -> BlobTapeAppender<'a> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        let tape = &self.tapes.tapes[i];

        BlobTapeAppender {
            // Safety: We are holding a write lock and only writers will drop tapes.
            backing_file: unsafe { &*tape.load(Ordering::Acquire) },
            resized_backing_file: &mut self.resized_tapes[i],
            min_resize: self.min_resize,
            current_used_bytes: self.meta_guard.tables_len_mut()[i],
            bytes_added: &mut self.added_bytes[i],
        }
    }

    /// Flush the changes to the database.
    ///
    /// Using a [`Flush`] mode other than [`Flush::Sync`] can leave the database in an invalid state if a crash
    /// happens before all changes are flushed to permanent storage.
    pub fn flush(mut self, mode: Flush) -> io::Result<()> {
        // First check if any resize happened.
        let mut resize_happened = false;
        for (i, resized_tape) in self.resized_tapes.iter_mut().enumerate() {
            if let Some(resized_tape) = resized_tape.take() {
                let resized_tape = Box::into_raw(Box::new(resized_tape));

                // We use `Release` here to prevent the allocation above from being reordered after this store.
                let old_tape = self.tapes.tapes[i].swap(resized_tape, Ordering::Release);
                let null = self.tapes.old_tapes[i].swap(old_tape, Ordering::Relaxed);

                // Can't happen, but make sure we don't leak any data.
                assert!(null.is_null());

                resize_happened = true;
            }
        }

        if resize_happened {
            // Tell the next write it needs to drop the old tape, this is synchronised with the write lock
            // so `Relaxed` is fine.
            self.tapes
                .old_tapes_need_flush
                .store(true, Ordering::Relaxed);
        }

        // flush each tapes changes to disk.
        // `Acquire` so we don't see the pointer before the allocation is done.
        match mode {
            Flush::Sync => {
                for (i, tape) in self.tapes.tapes.iter().enumerate() {
                    if self.added_bytes[i] != 0 {
                        // Safety: We are holding a write lock and only writers will drop tapes.
                        unsafe { &*tape.load(Ordering::Acquire) }.flush_range(
                            self.meta_guard.tables_len_mut()[i],
                            self.added_bytes[i],
                        )?;
                    }
                }
            }
            Flush::Async => {
                for (i, tape) in self.tapes.tapes.iter().enumerate() {
                    if self.added_bytes[i] != 0 {
                        // Safety: We are holding a write lock and only writers will drop tapes.
                        unsafe { &*tape.load(Ordering::Acquire) }.flush_range_async(
                            self.meta_guard.tables_len_mut()[i],
                            self.added_bytes[i],
                        )?;
                    }
                }
            }
            Flush::NoSync => {}
        }

        // Updated the length of each table in the metadata.
        for (len, added_bytes) in self
            .meta_guard
            .tables_len_mut()
            .iter_mut()
            .zip(&self.added_bytes)
        {
            *len += added_bytes;
        }

        // push the update for readers to see.
        self.meta_guard.push_update(mode)
    }
}

/// A popper for a [`LinearTapes`] database, allows atomically popping data from multiple tapes.
///
/// To pop data from a tape you must open a popper for that tape:
/// - for fixed sized tapes: [`Popper::fixed_sized_tape_popper`]
/// - for blob tapes: [`Popper::blob_tape_popper`]
///
/// Once finished changes must be committed with: [`Popper::flush`]
pub struct Popper<'a> {
    /// The metadata guard for this writer.
    meta_guard: metadata::MetadataWriteGuard<'a>,
    /// The tapes database.
    tapes: &'a LinearTapes,
}

impl Popper<'_> {
    /// Opens a handle to pop from a fixed-sized tape.
    pub fn fixed_sized_tape_popper<'a, E: Entry>(
        &'a mut self,
        table_name: &'static str,
    ) -> FixedSizedTapePopper<'a, E> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        let tape = &self.tapes.tapes[i];

        FixedSizedTapePopper {
            // Safety: We are holding a write lock and only writers will drop tapes.
            backing_file: unsafe { &*tape.load(Ordering::Acquire) },
            phantom: Default::default(),
            current_used_bytes: &mut self.meta_guard.tables_len_mut()[i],
        }
    }

    /// Opens a handle to pop from a blob tape.
    pub fn blob_tape_popper<'a>(&'a mut self, table_name: &'static str) -> BlobTapePopper<'a> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        let tape = &self.tapes.tapes[i];

        BlobTapePopper {
            // Safety: We are holding a write lock and only writers will drop tapes.
            backing_file: unsafe { &*tape.load(Ordering::Acquire) },
            current_used_bytes: &mut self.meta_guard.tables_len_mut()[i],
        }
    }

    /// Opens a handle to read from a fixed-sized tape.
    pub fn fixed_sized_tape_reader<'a, E: Entry>(
        &'a self,
        table_name: &'static str,
    ) -> FixedSizedTapeReader<'a, E> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        let tape = &self.tapes.tapes[i];

        FixedSizedTapeReader {
            // Safety: We are holding a write lock and only writers will drop tapes.
            backing_file: unsafe { &*tape.load(Ordering::Acquire) },
            phantom: Default::default(),
            len: self.meta_guard.tables_len()[i] / E::SIZE,
        }
    }

    /// Opens a handle to read from a blob tape.
    pub fn blob_tape_tape_reader<'a>(&'a self, table_name: &'static str) -> BlobTapeReader<'a> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        let tape = &self.tapes.tapes[i];

        BlobTapeReader {
            // Safety: We are holding a write lock and only writers will drop tapes.
            backing_file: unsafe { &*tape.load(Ordering::Acquire) },
            used_bytes: self.meta_guard.tables_len()[i],
        }
    }

    /// Flush the changes to the database.
    ///
    /// Using a [`Flush`] mode other than [`Flush::Sync`] can leave the database in an invalid state if a crash
    /// happens before all changes are flushed to permanent storage.
    pub fn flush(mut self, mode: Flush) -> io::Result<()> {
        // The poppers update the count directly + we don't need to flush anything to the files.
        self.meta_guard.push_update(mode)
    }
}

/// A [`LinearTapes`] database reader.
///
/// This type holds a read handle to the database so should not be kept around for longer than necessary.
pub struct Reader<'a> {
    /// A write guard for the metadata
    meta_guard: metadata::MetadataHandle<'a>,
    /// The ptrs to the tapes, guaranteed to be safe to read from, upto the lengths in the metadata.
    loaded_tapes: Vec<*const UnsafeTape>,
    /// The tapes.
    tapes: &'a LinearTapes,
}

unsafe impl Sync for Reader<'_> {}
unsafe impl Send for Reader<'_> {}

impl Reader<'_> {
    /// Opens a handle to read from a fixed-sized tape.
    pub fn fixed_sized_tape_reader<'a, E: Entry>(
        &'a self,
        table_name: &'static str,
    ) -> FixedSizedTapeReader<'a, E> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        // Safety: We are holding a reader guard so this will not be dropped.
        let backing_file = unsafe { &*self.loaded_tapes[i] };

        FixedSizedTapeReader {
            backing_file,
            phantom: Default::default(),
            len: self.meta_guard[i] / E::SIZE,
        }
    }

    /// Opens a handle to read from a blob tape.
    pub fn blob_tape_tape_reader<'a>(&'a self, table_name: &'static str) -> BlobTapeReader<'a> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        // Safety: We are holding a reader guard so this will not be dropped.
        let backing_file = unsafe { &*self.loaded_tapes[i] };

        BlobTapeReader {
            backing_file,
            used_bytes: self.meta_guard[i],
        }
    }
}
