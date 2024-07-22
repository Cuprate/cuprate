//! TODO

//---------------------------------------------------------------------------------------------------- Import
use serde::{Deserialize, Serialize};

use cuprate_rpc_types::json::GetBlockRequest;

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
#[derive(Deserialize, Serialize)]
#[serde(tag = "method", content = "params")]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// TODO
    GetBlock(GetBlockRequest),
}

impl Method {
    /// TODO
    pub const fn is_restricted(&self) -> bool {
        match self {
            Self::GetBlock(_) => false,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
