//! TODO
#![allow(clippy::unused_async)] // TODO: remove after impl

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::Cow;

use axum::{extract::State, http::StatusCode, Json};
use cuprate_json_rpc::{
    error::{ErrorCode, ErrorObject},
    Id,
};

use cuprate_rpc_types::{
    json::{JsonRpcRequest, JsonRpcResponse},
    RpcRequest,
};

use crate::{response::Response, rpc_handler::RpcHandler};

//---------------------------------------------------------------------------------------------------- Routes
/// TODO
pub(crate) async fn json_rpc<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<cuprate_json_rpc::Request<JsonRpcRequest>>,
) -> Result<Json<cuprate_json_rpc::Response<JsonRpcResponse>>, StatusCode> {
    // Return early if this RPC server is restricted and
    // the requested method is only for non-restricted RPC.
    if request.body.is_restricted() && handler.restricted() {
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

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
