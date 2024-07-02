//! The base data that appear in many RPC request/responses.
//!
//! These are the common "headers" or "base" types that are
//! [`flattened`](https://serde.rs/field-attrs.html#flatten)
//! into many of Monero's RPC types.
//!
//! The `Access*` structs (e.g. [`AccessResponseBase`]
//! are pseudo-deprecated structs for the RPC payment system, see:
//!
//! - <https://github.com/monero-project/monero/commit/2899379791b7542e4eb920b5d9d58cf232806937>
//! - <https://github.com/monero-project/monero/issues/8722>
//! - <https://github.com/monero-project/monero/pull/8843>

//---------------------------------------------------------------------------------------------------- Import
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::epee_object;

use crate::Status;

//---------------------------------------------------------------------------------------------------- Macro
/// Link the original `monerod` definition for RPC base types.
macro_rules! monero_rpc_base_link {
    ($start:literal..=$end:literal) => {
        concat!(
            "[Definition](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L",
            stringify!($start),
            "-L",
            stringify!($end),
            ")."
        )
    };
}

//---------------------------------------------------------------------------------------------------- Requests
/// The most common base for responses (nothing).
///
#[doc = monero_rpc_base_link!(95..=99)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EmptyRequestBase;

#[cfg(feature = "epee")]
epee_object! {
    EmptyRequestBase,
}

/// A base for RPC request types that support RPC payment.
///
#[doc = monero_rpc_base_link!(114..=122)]
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AccessRequestBase {
    /// The RPC payment client.
    pub client: String,
}

#[cfg(feature = "epee")]
epee_object! {
    AccessRequestBase,
    client: String,
}

//---------------------------------------------------------------------------------------------------- Responses
/// An empty response base.
///
/// This is for response types that do not contain
/// any extra fields, e.g. TODO.
// [`CalcPowResponse`](crate::json::CalcPowResponse).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EmptyResponseBase;

#[cfg(feature = "epee")]
epee_object! {
    EmptyResponseBase,
}

/// The most common base for responses.
///
#[doc = monero_rpc_base_link!(101..=112)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResponseBase {
    /// General RPC error code. [`Status::Ok`] means everything looks good.
    pub status: Status,
    /// States if the result is obtained using the bootstrap mode,
    /// and is therefore not trusted (`true`), or when the daemon
    /// is fully synced and thus handles the RPC locally (`false`).
    pub untrusted: bool,
}

#[cfg(feature = "epee")]
epee_object! {
    ResponseBase,
    status: Status,
    untrusted: bool,
}

/// A base for RPC response types that support RPC payment.
///
#[doc = monero_rpc_base_link!(124..=136)]
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AccessResponseBase {
    /// A flattened [`ResponseBase`].
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub response_base: ResponseBase,
    /// If payment for RPC is enabled, the number of credits
    /// available to the requesting client. Otherwise, `0`.
    pub credits: u64,
    /// If payment for RPC is enabled, the hash of the
    /// highest block in the chain. Otherwise, empty.
    pub top_hash: String,
}

#[cfg(feature = "epee")]
epee_object! {
    AccessResponseBase,
    credits: u64,
    top_hash: String,
    !flatten: response_base: ResponseBase,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
