#![doc = include_str!("../README.md")]
#![forbid(
    clippy::missing_assert_message,
    clippy::missing_errors_doc,
    clippy::should_panic_without_expect,
    clippy::single_char_lifetime_names,
    missing_docs,
    unsafe_code,
    missing_copy_implementations,
    reason = "Crate-specific lints. There should be good reasoning when removing these."
)]

mod reader_threads;
mod service;

pub use reader_threads::{init_thread_pool, ReaderThreads};

pub use service::{DatabaseReadService, DatabaseWriteHandle};
