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
pub(crate) async fn json_rpc<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<cuprate_json_rpc::Request<JsonRpcRequest>>,
) -> Result<Json<cuprate_json_rpc::Response<JsonRpcResponse>>, StatusCode> {
    // Return early if this RPC server is restricted and
    // the requested method is only for non-restricted RPC.
    if handler.state().restricted() && request.body.is_restricted() {
        let error_object = ErrorObject {
            code: ErrorCode::ServerError(-1 /* TODO */),
            message: Cow::Borrowed("Restricted. TODO"),
            data: None,
        };

        // JSON-RPC 2.0 rule:
        // If there was an error in detecting the `Request`'s ID,
        // the `Response` must contain an `Id::Null`
        let id = request.id.unwrap_or(Id::Null);

        let response = cuprate_json_rpc::Response::err(id, error_object);

        // TODO
        return Ok(Json(response));
    }

    // TODO: call handler
    let Response::JsonRpc(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn binary<H: RpcHandler>(
    State(handler): State<H>,
    endpoint: &'static str,
    mut request: Bytes, // TODO: BinRequest
) -> Result<BinResponse, StatusCode> {
    let error = |_| StatusCode::INTERNAL_SERVER_ERROR;

    let request = match endpoint {
        "/get_blocks.bin" | "/getblocks.bin" => {
            BinRequest::GetBlocks(from_bytes(&mut request).map_err(error)?)
        }
        "/get_blocks_by_height.bin" | "/getblocks_by_height.bin" => {
            BinRequest::GetBlocksByHeight(from_bytes(&mut request).map_err(error)?)
        }
        "/get_hashes.bin" | "/gethashes.bin" => {
            BinRequest::GetHashes(from_bytes(&mut request).map_err(error)?)
        }
        "/get_o_indexes.bin" => {
            BinRequest::GetOutputIndexes(from_bytes(&mut request).map_err(error)?)
        }
        "/get_outs.bin" => BinRequest::GetOuts(from_bytes(&mut request).map_err(error)?),
        "/get_transaction_pool_hashes.bin" => BinRequest::GetTransactionPoolHashes(()),
        "/get_output_distribution.bin" => {
            BinRequest::GetOutputDistribution(from_bytes(&mut request).map_err(error)?)
        }

        // INVARIANT:
        // The `create_router` function only passes the above endpoints.
        _ => unreachable!(),
    };

    // TODO
    if handler.state().restricted() && request.is_restricted() {
        return Err(StatusCode::NOT_FOUND);
    }

    // TODO: call handler
    let Response::Binary(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(response)
}

/// TODO
pub(crate) async fn other<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<OtherRequest>,
) -> Result<Json<OtherResponse>, StatusCode> {
    if handler.state().restricted() && request.is_restricted() {
        todo!();
    }

    // TODO: call handler
    let Response::Other(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

/// TODO
pub(crate) async fn unknown() -> StatusCode {
    StatusCode::NOT_FOUND
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
