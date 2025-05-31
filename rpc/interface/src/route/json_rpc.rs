//! JSON-RPC 2.0 endpoint route functions.

//---------------------------------------------------------------------------------------------------- Import
use axum::{extract::State, http::StatusCode, Json};
use bytes::{BufMut, Bytes, BytesMut};
use serde_json::ser::{CompactFormatter, Formatter};
use tower::ServiceExt;

use cuprate_json_rpc::{Id, Response};
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
) -> Result<Json<Bytes>, StatusCode> {
    let json_formatter = handler.json_formatter();

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
    //
    // INVARIANT:
    // The RPC handler functions in `cuprated` depend on this line existing,
    // the functions themselves do not check if they are being called
    // from an (un)restricted context. This line must be here or all
    // methods will be allowed to be called freely.
    if request.body.is_restricted() && handler.is_restricted() {
        match json_formatter.to_bytes(&Response::<()>::method_not_found(id)) {
            Ok(json) => {
                // The error when a restricted JSON-RPC method is called as per:
                //
                // - <https://github.com/monero-project/monero/blob/893916ad091a92e765ce3241b94e706ad012b62a/contrib/epee/include/net/http_server_handlers_map2.h#L244-L252>
                // - <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L188>
                return Ok(Json(json));
            }
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        };
    }

    // Send request.
    let Ok(response) = handler.oneshot(request.body).await else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let Ok(json) = json_formatter.to_bytes(&Response::ok(id, response)) else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    Ok(Json(json))
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
