#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(
    clippy::allow_attributes,
    reason = "macros (internal + serde) make this lint hard to satisfy"
)]

mod constants;
#[cfg(any(feature = "serde", feature = "epee"))]
mod defaults;
mod free;
#[cfg(feature = "from")]
mod from;
mod macros;
mod rpc_call;

#[cfg(feature = "serde")]
mod serde;

pub mod base;
pub mod bin;
pub mod json;
pub mod misc;
pub mod other;

pub use constants::{
    CORE_RPC_STATUS_BUSY, CORE_RPC_STATUS_NOT_MINING, CORE_RPC_STATUS_OK,
    CORE_RPC_STATUS_PAYMENT_REQUIRED, CORE_RPC_VERSION, CORE_RPC_VERSION_MAJOR,
    CORE_RPC_VERSION_MINOR,
};
pub use rpc_call::{RpcCall, RpcCallValue};

// false-positive: used in tests
#[cfg(test)]
mod test {
    extern crate cuprate_test_utils;
    extern crate serde_json;
}
