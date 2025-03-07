#![doc = include_str!("../README.md")]

mod reader_threads;
mod service;

pub use reader_threads::{ReaderThreads, init_thread_pool};

pub use service::{DatabaseReadService, DatabaseWriteHandle};
