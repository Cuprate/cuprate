//! Monero RPC data.
//!
//! This module contains real `monerod` RPC requests/responses
//! as `const` [`str`]s and byte arrays (binary).
//!
//! The strings include the JSON-RPC 2.0 portions of the JSON.
//! - Tests exist within this crate that ensure the JSON is valid
//! - Tests exist within Cuprate's `rpc/` crates that ensure these strings (de)serialize as valid types
//!
//! # Determinism
//! Note that although both request/response data is defined,
//! they aren't necessarily tied to each other, i.e. the request
//! will not deterministically lead to the response.

pub mod bin;
pub mod json;
mod macros;
pub mod other;
