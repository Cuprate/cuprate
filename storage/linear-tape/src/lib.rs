//! Tape Databases
//!
//! This crate implements multiple tape databases. A tape database is a database which stores items
//! contiguously in the order they are inserted. It supports pushing and popping from the top of the
//! database.
//!
//! The 2 databases currently exposed:
//! - [`LinearFixedSizeTape`] stores fixed sized values and allows direct indexing to get values.
//! - [`LinearBlobTape`] stores raw bytes, you must store the index the values are at to retrieve them.
//!
//! # Safety
//!
//! All databases are backed by a memory map, which practically can not be worked on in Rust in completely safe code.
//! So you must make sure the file is not edited in a way that is not allowed by this crate.
//!
//! All databases support multiple readers and a single appender, to pop you must have exclusive access to
//! the database. These requirements are not enforced for multiple instances of the same database in or out
//! of process, you must ensure them yourself.
//!
//! All functions marked as safe in this crate _are_ safe as long as there is only 1 instance of the database,
//! with more you must use a synchronisation mechanism to enforce the requirements.
//!
//!
#![allow(unreachable_pub)]

mod blob_tape;
mod fixed_size;
mod meta;
mod unsafe_tape;

use crate::meta::{MetadataFile, WriteOp};
pub use blob_tape::*;
pub use fixed_size::*;
use std::collections::HashMap;
use std::fs::{create_dir, create_dir_all};
use std::io;
use std::path::{Path, PathBuf};
pub(crate) use unsafe_tape::*;

/// Advice to give the OS when opening the memory map file.
pub enum Advice {
    /// [`memmap2::Advice::Normal`]
    Normal,
    /// [`memmap2::Advice::Random`]
    Random,
    /// [`memmap2::Advice::Sequential`]
    Sequential,
}

impl Advice {
    fn to_memmap2_advice(&self) -> memmap2::Advice {
        match self {
            Advice::Normal => memmap2::Advice::Normal,
            Advice::Random => memmap2::Advice::Random,
            Advice::Sequential => memmap2::Advice::Sequential,
        }
    }
}

#[derive(Copy, Clone)]
pub enum Flush {
    Sync,
    Async,
    NoSync,
}

#[derive(Copy, Clone, Debug)]
pub struct ResizeNeeded;

pub struct LinearTapes {
    tapes_to_index: HashMap<&'static str, usize>,
    metadata: MetadataFile,

    tapes: Vec<UnsafeTape>,
}

pub struct Tapes {
    pub name: &'static str,
    pub path: Option<PathBuf>,
}

impl LinearTapes {
    pub unsafe fn new<P: AsRef<Path>>(
        tapes: Vec<Tapes>,
        metadata_path: P,
    ) -> Result<Self, io::Error> {
        create_dir_all(metadata_path.as_ref())?;

        let metadata = unsafe {
            MetadataFile::open(
                metadata_path.as_ref().join("metadata.tapes"),
                tapes.len(),
                8,
            )?
        };

        let default_tape_path = metadata_path.as_ref().join("tapes");

        create_dir(default_tape_path.as_path())?;

        let tapes_to_index = tapes.iter().enumerate().map(|(i, t)| (t.name, i)).collect();

        let tapes = tapes
            .iter()
            .map(|tape| {
                let path = tape.path.as_ref().unwrap_or(&default_tape_path);

                unsafe {
                    UnsafeTape::open(
                        path.join(format!("{}.tape", tape.name)),
                        Advice::Sequential,
                        40 * 1024 * 1024 * 1024,
                    )
                }
            })
            .collect::<Result<_, _>>()?;

        Ok(LinearTapes {
            tapes_to_index,
            metadata,
            tapes,
        })
    }

    pub fn appender(&self) -> Appender<'_> {
        Appender {
            meta_guard: self.metadata.start_write(WriteOp::Push),
            tapes: &self,
            added_bytes: vec![0; self.tapes.len()],
        }
    }

    pub fn reader(&self) -> Reader<'_> {
        Reader {
            meta_guard: self.metadata.start_read(),
            tapes: &self,
        }
    }
}

pub struct Appender<'a> {
    meta_guard: meta::WriteGuard<'a>,
    tapes: &'a LinearTapes,
    added_bytes: Vec<usize>,
}

impl Appender<'_> {
    pub fn fixed_sized_tape_appender<'a, E: Entry>(
        &'a mut self,
        table_name: &'static str,
    ) -> LinearTapeAppender<'a, E> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        let tape = &self.tapes.tapes[i];

        LinearTapeAppender {
            backing_file: tape,
            phantom: Default::default(),
            current_used_bytes: self.meta_guard.tables_len_mut()[i],
            entries_added: &mut self.added_bytes[i],
        }
    }

    pub fn blob_tape_appender<'a>(
        &'a mut self,
        table_name: &'static str,
    ) -> LinearBlobTapeAppender<'a> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        let tape = &self.tapes.tapes[i];

        LinearBlobTapeAppender {
            backing_file: tape,
            current_used_bytes: self.meta_guard.tables_len_mut()[i],
            bytes_added: &mut self.added_bytes[i],
        }
    }

    pub fn flush(mut self, mode: Flush) -> io::Result<()> {
        match mode {
            Flush::Sync => {
                for (i, tape) in self.tapes.tapes.iter().enumerate() {
                    tape.flush_range(self.meta_guard.tables_len_mut()[i], self.added_bytes[i])?;
                }
            }
            Flush::Async => {
                for (i, tape) in self.tapes.tapes.iter().enumerate() {
                    tape.flush_range_async(
                        self.meta_guard.tables_len_mut()[i],
                        self.added_bytes[i],
                    )?;
                }
            }
            Flush::NoSync => {}
        }

        for (len, added_bytes) in self
            .meta_guard
            .tables_len_mut()
            .iter_mut()
            .zip(&self.added_bytes)
        {
            *len += added_bytes
        }

        self.meta_guard.push_update();

        Ok(())
    }
}

pub struct Reader<'a> {
    meta_guard: meta::MetadataHandle<'a>,
    tapes: &'a LinearTapes,
}

impl Reader<'_> {
    pub fn fixed_sized_tape_reader<'a, E: Entry>(
        &'a self,
        table_name: &'static str,
    ) -> LinearTapeReader<'a, E> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        let tape = &self.tapes.tapes[i];

        LinearTapeReader {
            backing_file: tape,
            phantom: Default::default(),
            len: self.meta_guard.tables_len[i],
        }
    }

    pub fn blob_tape_tape_reader<'a>(
        &'a self,
        table_name: &'static str,
    ) -> LinearBlobTapeReader<'a> {
        let i = *self
            .tapes
            .tapes_to_index
            .get(table_name)
            .expect("Tape was not specified when opening tapes");

        let tape = &self.tapes.tapes[i];

        LinearBlobTapeReader {
            backing_file: tape,
            used_bytes: self.meta_guard.tables_len[i],
        }
    }
}
