//! JSON-RPC 2.0 endpoint route functions.

//---------------------------------------------------------------------------------------------------- Import
use axum::{
    body::Body,
    extract::State,
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use tower::ServiceExt;

use cuprate_json_rpc::{Id, Response as JsonRpcResponse};
use cuprate_rpc_types::{
    json::{GetTxpoolBacklogResponse, JsonRpcRequest, JsonRpcResponse as RpcResponse},
    RpcCallValue,
};

use crate::rpc_handler::RpcHandler;

//---------------------------------------------------------------------------------------------------- Routes
/// The `/json_rpc` route function used in [`crate::RouterBuilder`].
pub(crate) async fn json_rpc<H: RpcHandler>(
    State(handler): State<H>,
    Json(request): Json<cuprate_json_rpc::Request<JsonRpcRequest>>,
) -> Result<Response, StatusCode> {
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
        // The error when a restricted JSON-RPC method is called as per:
        //
        // - <https://github.com/monero-project/monero/blob/893916ad091a92e765ce3241b94e706ad012b62a/contrib/epee/include/net/http_server_handlers_map2.h#L244-L252>
        // - <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L188>
        return Ok(Json(JsonRpcResponse::<RpcResponse>::method_not_found(id)).into_response());
    }

    // Send request.
    let Ok(response) = handler.oneshot(request.body).await else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    if let RpcResponse::GetTxpoolBacklog(response) = &response {
        return txpool_backlog_response(&id, response);
    }

    Ok(Json(JsonRpcResponse::ok(id, response)).into_response())
}

// TODO: remove the code below once this: https://github.com/monero-project/monero/issues/9422 is resolved.

/// Build a Monero-compatible `get_txpool_backlog` response.
fn txpool_backlog_response(
    id: &Id,
    response: &GetTxpoolBacklogResponse,
) -> Result<Response, StatusCode> {
    let body = serialize_txpool_backlog_response(id, response)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Response::builder()
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("valid response builder"))
}

/// Serialize the backlog's POD container as a binary JSON string.
fn serialize_txpool_backlog_response(
    id: &Id,
    response: &GetTxpoolBacklogResponse,
) -> serde_json::Result<Vec<u8>> {
    let mut json = [
        br#"{
            "jsonrpc":"2.0",
            "id":"#,
        serde_json::to_string(&id)?.as_bytes(),
        br#",
            "result":"#,
        serde_json::to_string(&response.base)?.as_bytes(),
    ]
    .concat();

    if !response.backlog.is_empty() {
        // Reopen `result` to append the field that serde cannot represent.
        let Some(b'}') = json.pop() else {
            unreachable!("response base must serialize as a JSON object");
        };
        json.extend_from_slice(br#","backlog":"#);
        write_binary_string(&mut json, &txpool_backlog_blob(response));
        json.push(b'}');
    }

    json.push(b'}');
    Ok(json)
}

fn txpool_backlog_blob(response: &GetTxpoolBacklogResponse) -> Vec<u8> {
    const ENTRY_SIZE: usize = size_of::<u64>() * 3;

    let mut blob = Vec::with_capacity(response.backlog.len() * ENTRY_SIZE);

    for entry in &response.backlog {
        // monerod does to_ne_bytes here, which means be nodes and le wallets can't communicate,
        // we just use le.
        blob.extend_from_slice(&entry.weight.to_le_bytes());
        blob.extend_from_slice(&entry.fee.to_le_bytes());
        blob.extend_from_slice(&entry.time_in_pool.to_le_bytes());
    }

    blob
}

/// This function implements monerod's bad JSON byte writing.
/// The function in monerod is here: <https://github.com/monero-project/monero/blob/641e5ca588c7babef9d65b1cbae63970fb5aba12/contrib/epee/src/parserse_base_utils.cpp#L42-L92>
fn write_binary_string(json: &mut Vec<u8>, bytes: &[u8]) {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

    json.push(b'"');
    for &byte in bytes {
        match byte {
            b'\x08' => json.extend_from_slice(br"\b"),
            b'\x0c' => json.extend_from_slice(br"\f"),
            b'\n' => json.extend_from_slice(br"\n"),
            b'\r' => json.extend_from_slice(br"\r"),
            b'\t' => json.extend_from_slice(br"\t"),
            b'"' => json.extend_from_slice(br#"\""#),
            b'\\' => json.extend_from_slice(br"\\"),
            b'/' => json.extend_from_slice(br"\/"),
            0..=0x1f => {
                json.extend_from_slice(br"\u00");
                json.push(HEX_DIGITS[usize::from(byte >> 4)]);
                json.push(HEX_DIGITS[usize::from(byte & 0x0f)]);
            }
            _ => json.push(byte),
        }
    }
    json.push(b'"');
}
