#![doc = include_str!("../README.md")]
#![forbid(
    clippy::missing_assert_message,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::should_panic_without_expect,
    missing_docs,
    unsafe_code,
    missing_copy_implementations,
    missing_debug_implementations,
    reason = "Crate-specific lints. There should be good reasoning when removing these."
)]

pub mod error;

mod id;
pub use id::Id;

mod version;
pub use version::Version;

mod request;
pub use request::Request;

mod response;
pub use response::Response;

#[cfg(test)]
mod tests;
