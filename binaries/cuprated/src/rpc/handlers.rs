//! RPC handler functions.
//!
//! These are the glue (async) functions that connect all the
//! internal `cuprated` functions and fulfill the request.
//!
//! - JSON-RPC handlers are in [`json_rpc`]
//! - Other JSON endpoint handlers are in [`other_json`]
//! - Other binary endpoint handlers are in [`bin`]
//!
//! - [`helper`] contains helper functions used by many handlers
//! - [`shared`] contains shared functions used by multiple handlers

pub(super) mod bin;
pub(super) mod json_rpc;
pub(super) mod other_json;

mod helper;
mod shared;
