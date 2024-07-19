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
//!
//! Note that this library doesn't use [`AccessRequestBase`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L114-L122) found in `monerod`
//! as the type is practically deprecated.
//!
//! Although, [`AccessResponseBase`] still exists as to allow
//! outputting the same JSON fields as `monerod` (even if deprecated).

//---------------------------------------------------------------------------------------------------- Import
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::epee_object;

use crate::{macros::monero_definition_link, misc::Status};

//---------------------------------------------------------------------------------------------------- Requests
/// A base for RPC request types that support RPC payment.
///
#[doc = monero_definition_link!(cc73fe71162d564ffda8e549b79a350bca53c454, "rpc/core_rpc_server_commands_defs.h", 114..=122)]
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
#[doc = monero_definition_link!(cc73fe71162d564ffda8e549b79a350bca53c454, "rpc/core_rpc_server_commands_defs.h", 101..=112)]
/// The most common base for responses.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResponseBase {
    /// General RPC error code. [`Status::Ok`] means everything looks good.
    pub status: Status,
    /// States if the result is obtained using the bootstrap mode,
    /// and is therefore not trusted (`true`), or when the daemon
    /// is fully synced and thus handles the RPC locally (`false`).
    pub untrusted: bool,
}

impl ResponseBase {
    /// TODO
    pub const fn ok() -> Self {
        Self {
            status: Status::Ok,
            untrusted: false,
        }
    }

    /// TODO
    pub const fn ok_untrusted() -> Self {
        Self {
            status: Status::Ok,
            untrusted: true,
        }
    }
}

#[cfg(feature = "epee")]
epee_object! {
    ResponseBase,
    status: Status,
    untrusted: bool,
}

#[doc = monero_definition_link!(cc73fe71162d564ffda8e549b79a350bca53c454, "rpc/core_rpc_server_commands_defs.h", 124..=136)]
/// A base for RPC response types that support RPC payment.
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

impl AccessResponseBase {
    /// TODO
    pub const fn ok() -> Self {
        Self {
            response_base: ResponseBase::ok(),
            credits: 0,
            top_hash: String::new(),
        }
    }

    /// TODO
    pub const fn ok_untrusted() -> Self {
        Self {
            response_base: ResponseBase::ok_untrusted(),
            credits: 0,
            top_hash: String::new(),
        }
    }
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
