#![doc = include_str!("../README.md")]
#![deny(missing_docs, reason = "all constants should document what they are")]

mod macros;

#[cfg(feature = "block")]
pub mod block;
#[cfg(feature = "build")]
pub mod build;
#[cfg(feature = "output")]
pub mod output;
#[cfg(feature = "rpc")]
pub mod rpc;
