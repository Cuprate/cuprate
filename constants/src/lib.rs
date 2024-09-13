#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

mod macros;

#[cfg(feature = "cryptonote")]
pub mod cryptonote;
