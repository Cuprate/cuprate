//! Monero RPC data.
//!
//! This module contains real `monerod` RPC
//! requests/responses as `const` strings.
//!
//! These strings include the JSON-RPC 2.0 portions of the JSON.
//!
//! Tests exist within Cuprate's `rpc/` crates that
//! ensure these strings are valid.
//!
//! # Determinism
//! Note that although both request/response data is defined,
//! they aren't necessarily tied to each other, i.e. the request
//! will not deterministically lead to the response.

pub mod bin;
pub mod json;
mod macros;
pub mod other;
