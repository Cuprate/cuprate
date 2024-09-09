//! JSON-RPC 2.0 endpoint route functions.

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::Cow;

use axum::{extract::State, http::StatusCode, Json};
use tower::ServiceExt;

use cuprate_json_rpc::{
    error::{ErrorCode, ErrorObject},
    Id, Response,
};
use cuprate_rpc_types::{
    json::{JsonRpcRequest, JsonRpcResponse},
    RpcCallValue,
};

use crate::rpc_handler::RpcHandler;

//---------------------------------------------------------------------------------------------------- Routes
/// The `/json_rpc` route function used in [`crate::RouterBuilder`].
pub(crate) async fn json_rpc<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<cuprate_json_rpc::Request<JsonRpcRequest>>,
) -> Result<Json<Response<JsonRpcResponse>>, StatusCode> {
    // TODO: <https://www.jsonrpc.org/specification#notification>
    //
    // JSON-RPC notifications (requests without `id`)
    // must not be responded too, although, the request's side-effects
    // must remain. How to do this considering this function will
    // always return and cause `axum` to respond?

    // JSON-RPC 2.0 rule:
    // If there was an error in detecting the `Request`'s ID,
    // the `Response` must contain an `Id::Null`
    let id = request.id.unwrap_or(Id::Null);

    // Return early if this RPC server is restricted and
    // the requested method is only for non-restricted RPC.
    if request.body.is_restricted() && handler.restricted() {
        let error_object = ErrorObject {
            code: ErrorCode::ServerError(-1 /* TODO */),
            message: Cow::Borrowed("Restricted. TODO: mimic monerod message"),
            data: None,
        };

        let response = Response::err(id, error_object);

        return Ok(Json(response));
    }

    // Send request.
    let Ok(response) = handler.oneshot(request.body).await else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(Json(Response::ok(id, response)))
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
