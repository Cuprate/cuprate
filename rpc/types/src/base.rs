//! The base data that appear in many RPC request/responses.
//!
//! TODO

//---------------------------------------------------------------------------------------------------- Import
use serde::{Deserialize, Serialize};

use cuprate_epee_encoding::epee_object;

use crate::Status;

//---------------------------------------------------------------------------------------------------- Macro
/// TODO
macro_rules! doc_monero_base_rpc_link {
    ($start:literal..=$end:literal) => {
        concat!(
            "https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L",
            stringify!($start),
            "-L",
            stringify!($end),
        )
    };
}

//---------------------------------------------------------------------------------------------------- Requests
/// TODO
///
#[doc = doc_monero_base_rpc_link!(95..=99)]
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct EmptyRequestBase;

cuprate_epee_encoding::epee_object! {
    EmptyRequestBase,
}

/// TODO
///
#[doc = doc_monero_base_rpc_link!(114..=122)]
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccessRequestBase {
    /// TODO
    pub client: String,
}

cuprate_epee_encoding::epee_object! {
    AccessRequestBase,
    client: String,
}

//---------------------------------------------------------------------------------------------------- Responses
/// TODO
///
#[doc = doc_monero_base_rpc_link!(101..=112)]
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct EmptyResponseBase;

cuprate_epee_encoding::epee_object! {
    EmptyResponseBase,
}

/// TODO
///
#[doc = doc_monero_base_rpc_link!(101..=112)]
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ResponseBase {
    /// TODO
    pub status: Status,
    /// TODO
    pub untrusted: bool,
}

epee_object! {
    ResponseBase,
    status: Status,
    untrusted: bool,
}

/// TODO
///
#[doc = doc_monero_base_rpc_link!(124..=136)]
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccessResponseBase {
    /// TODO
    #[serde(flatten)]
    pub response_base: ResponseBase,
    /// TODO
    pub credits: u64,
    /// TODO
    pub top_hash: String,
}

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
