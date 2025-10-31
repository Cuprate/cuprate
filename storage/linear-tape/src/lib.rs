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

mod blob_tape;
mod unsafe_tape;
mod fixed_size;

use std::io;
pub use blob_tape::*;
pub use fixed_size::*;
pub(crate) use unsafe_tape::*;

/// Advice to give the OS when opening the memory map file.
pub enum Advice {
    /// [`memmap2::Advice::Normal`]
    Normal,
    /// [`memmap2::Advice::Random`]
    Random,
    /// [`memmap2::Advice::Sequential`]
    Sequential
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
    NoSync
}

#[derive(Copy, Clone, Debug)]
pub struct ResizeNeeded;
