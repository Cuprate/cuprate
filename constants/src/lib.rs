#![doc = include_str!("../README.md")]
#![forbid(
    clippy::missing_assert_message,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::should_panic_without_expect,
    clippy::single_char_lifetime_names,
    missing_docs,
    unsafe_code,
    unused_results,
    missing_copy_implementations,
    missing_debug_implementations,
    reason = "Crate-specific lints. There should be good reasoning when removing these."
)]
#![no_std] // This can be removed if we eventually need `std`.

mod macros;

#[cfg(feature = "block")]
pub mod block;
#[cfg(feature = "build")]
pub mod build;
#[cfg(feature = "rpc")]
pub mod rpc;
