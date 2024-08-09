#![doc = include_str!("../README.md")]

mod reader_threads;
mod service;

pub use reader_threads::{init_thread_pool, ReaderThreads};

pub use service::{DatabaseReadService, DatabaseWriteHandle};
