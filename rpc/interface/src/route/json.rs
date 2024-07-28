//! TODO
#![allow(clippy::unused_async)] // TODO: remove after impl

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::Cow;

use axum::{extract::State, http::StatusCode, Json};
use tower::ServiceExt;

use cuprate_json_rpc::{
    error::{ErrorCode, ErrorObject},
    Id,
};
use cuprate_rpc_types::{
    json::{JsonRpcRequest, JsonRpcResponse},
    RpcCall,
};

use crate::{rpc_handler::RpcHandler, rpc_request::RpcRequest, rpc_response::RpcResponse};

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
            message: Cow::Borrowed("Restricted. TODO: mimic monerod message"),
            data: None,
        };

        // JSON-RPC 2.0 rule:
        // If there was an error in detecting the `Request`'s ID,
        // the `Response` must contain an `Id::Null`
        let id = request.id.unwrap_or(Id::Null);

        let response = cuprate_json_rpc::Response::err(id, error_object);

        return Ok(Json(response));
    }

    // Send request.
    let request = RpcRequest::JsonRpc(request);
    let channel = handler.oneshot(request).await?;

    // Assert the response from the inner handler is correct.
    let RpcResponse::JsonRpc(response) = channel else {
        panic!("RPC handler returned incorrect response");
    };

    Ok(Json(response))
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
