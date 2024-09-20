#![doc = include_str!("../README.md")]
#![deny(missing_docs, reason = "all constants should document what they are")]

cfg_if::cfg_if! {
    // Used in test modules.
    if #[cfg(test)] {
        use hex as _;
        use monero_serai as _;
        use pretty_assertions as _;
    }
}

mod macros;

#[cfg(feature = "block")]
pub mod block;
#[cfg(feature = "build")]
pub mod build;
#[cfg(feature = "genesis")]
pub mod genesis;
#[cfg(feature = "rpc")]
pub mod rpc;
