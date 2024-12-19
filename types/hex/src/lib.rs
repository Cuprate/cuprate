#![doc = include_str!("../README.md")]
// Allow some lints when running in debug mode.
#![cfg_attr(debug_assertions, allow(clippy::todo, clippy::multiple_crate_versions))]

mod array;
mod vec;

pub use array::Hex;
pub use vec::HexVec;
