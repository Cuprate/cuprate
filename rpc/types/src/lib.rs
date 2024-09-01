#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod constants;
mod defaults;
mod free;
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
