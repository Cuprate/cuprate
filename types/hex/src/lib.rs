#![doc = include_str!("../README.md")]
// Allow some lints when running in debug mode.
#![cfg_attr(debug_assertions, allow(clippy::todo, clippy::multiple_crate_versions))]

mod hex;

pub use hex::Hex;
