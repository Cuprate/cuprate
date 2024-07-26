#![allow(clippy::impl_trait_in_params)]

mod reader_threads;
mod service;

pub use reader_threads::{init_thread_pool, ReaderThreads};

pub use service::{DatabaseReadService, DatabaseWriteHandle};
