//! Monero RPC types.
//!
//! This module contains real `monerod` RPC
//! requests/responses as `const` strings.
//!
//! These strings include the JSON-RPC 2.0 portions of the JSON.
//!
//! Tests exist within Cuprate's `rpc/` crates that
//! ensure these strings are valid.

pub mod bin;
pub mod json;
mod macros;
pub mod other;
