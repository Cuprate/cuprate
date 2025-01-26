#![forbid(
    clippy::missing_assert_message,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::should_panic_without_expect,
    clippy::single_char_lifetime_names,
    unsafe_code,
    unused_results,
    missing_copy_implementations,
    missing_debug_implementations,
    reason = "Crate-specific lints. There should be good reasoning when removing these."
)]

pub mod json_message_types;
