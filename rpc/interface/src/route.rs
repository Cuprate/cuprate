//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, future::Future, sync::Arc};

use axum::{body::Bytes, extract::State, http::StatusCode, Json};
use cuprate_json_rpc::{
    error::{ErrorCode, ErrorObject},
    Id,
};
use tower::{Service, ServiceExt};

use cuprate_rpc_types::{
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
pub(crate) async fn bin<H: RpcHandler>(
    State(handler): State<H>,
    request: Bytes, // TODO: BinRequest
) -> Result<Vec<u8>, StatusCode> {
    // TODO
    // if handler.state().restricted() && request.is_restricted() {
    if handler.state().restricted() {
        return Err(StatusCode::NOT_FOUND);
    }

    // TODO: call handler
    let Response::Binary(response) = todo!() else {
        panic!("RPC handler returned incorrect response");
    };

    let binary: Vec<u8> = todo!(); // TODO: serialize response.

    Ok(binary)
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
