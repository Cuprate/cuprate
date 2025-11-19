#![doc = include_str!("../README.md")]
pub use cuprate_database::RuntimeError;

mod reader_threads;
mod service;

pub use reader_threads::{init_thread_pool, ReaderThreads};

pub use service::{DatabaseReadService, DatabaseWriteHandle};
