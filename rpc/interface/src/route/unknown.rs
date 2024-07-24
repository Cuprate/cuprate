//! TODO
#![allow(clippy::unused_async)] // TODO: remove after impl

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, future::Future, sync::Arc};

use axum::{body::Bytes, extract::State, http::StatusCode, Json};
use cuprate_json_rpc::{
    error::{ErrorCode, ErrorObject},
    Id,
};
use tower::{Service, ServiceExt};

use cuprate_epee_encoding::from_bytes;
use cuprate_rpc_types::{
    bin::{
        BinRequest, BinResponse, GetBlocksByHeightRequest, GetBlocksRequest, GetHashesRequest,
        GetOutputIndexesRequest, GetOutsRequest, GetTransactionPoolHashesRequest,
    },
    json::{JsonRpcRequest, JsonRpcResponse},
    other::{OtherRequest, OtherResponse},
    RpcRequest,
};

use crate::{
    error::Error, request::Request, response::Response, rpc_handler::RpcHandler,
    rpc_state::RpcState,
};

//---------------------------------------------------------------------------------------------------- Routes
/// TODO
pub(crate) async fn unknown() -> StatusCode {
    StatusCode::NOT_FOUND
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
