//! JSON-RPC 2.0 endpoint route functions.

//---------------------------------------------------------------------------------------------------- Import
use axum::{
    body::{Body, Bytes},
    extract::{rejection::BytesRejection, State},
    http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use futures::stream::{FuturesUnordered, StreamExt};
use serde::Deserialize;
use serde_json::value::RawValue;
use tower::ServiceExt;

use cuprate_json_rpc::{error::ErrorObject, Id, Response as JsonRpcResponse};
use cuprate_rpc_types::{
    json::{GetTxpoolBacklogResponse, JsonRpcMethod, JsonRpcResponse as RpcResponse},
    RpcCallValue,
};

use crate::rpc_handler::RpcHandler;

//---------------------------------------------------------------------------------------------------- Routes
/// The `/json_rpc` route function used in [`crate::RouterBuilder`].
pub(crate) async fn json_rpc<H: RpcHandler>(
    State(handler): State<H>,
    headers: HeaderMap,
    request: Result<Bytes, BytesRejection>,
) -> Response {
    if !json_content_type(&headers) {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }

    let request = match request {
        Ok(request) => request,
        Err(rejection) => return rejection.into_response(),
    };

    // A JSON array is a batch, anything else is a single request.
    if request.trim_ascii_start().starts_with(b"[") {
        return batch(handler, &request).await;
    }

    match dispatch(handler, &request).await {
        Some(response) => json_rpc_response(response),
        None => StatusCode::OK.into_response(),
    }
}

/// Handle a [batch](https://www.jsonrpc.org/specification#batch) of requests.
async fn batch<H: RpcHandler>(handler: H, request: &[u8]) -> Response {
    let entries = match serde_json::from_slice::<Vec<&RawValue>>(request) {
        // JSON-RPC 2.0 rule:
        // An empty batch is an invalid request.
        Ok(entries) if !entries.is_empty() => entries,
        Err(error) if error.is_syntax() || error.is_eof() => {
            return json_rpc_response(JsonRpcResponse::parse_error(Id::Null));
        }
        _ => {
            return json_rpc_response(JsonRpcResponse::invalid_request(Id::Null));
        }
    };

    let concurrency = handler.batch_concurrency().get();
    let mut entries = entries.into_iter();
    let mut responses = FuturesUnordered::new();
    for entry in entries.by_ref().take(concurrency) {
        responses.push(dispatch(handler.clone(), entry.get().as_bytes()));
    }

    let mut json = Vec::new();
    json.push(b'[');

    while let Some(response) = responses.next().await {
        if let Some(response) = response {
            write_response_json(response, &mut json);
            json.push(b',');
        }
        if let Some(entry) = entries.next() {
            responses.push(dispatch(handler.clone(), entry.get().as_bytes()));
        }
    }

    // A batch of only notifications is not responded to.
    if json.len() == 1 {
        return StatusCode::OK.into_response();
    }

    json.pop();
    json.push(b']');

    Response::builder()
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json))
        .expect("valid response builder")
}

/// Handle a single request, returning its serialized response.
///
/// [`None`] if the request was a
/// [notification](https://www.jsonrpc.org/specification#notification).
async fn dispatch<H: RpcHandler>(handler: H, json: &[u8]) -> Option<JsonRpcResponse<RpcResponse>> {
    let request = match serde_json::from_slice::<cuprate_json_rpc::Request<JsonRpcMethod>>(json) {
        Ok(request) => request,
        Err(error) if error.is_syntax() || error.is_eof() => {
            return Some(JsonRpcResponse::parse_error(Id::Null));
        }
        Err(_) => {
            // The request is malformed; recover its ID if it has a valid one, as per:
            // <https://www.jsonrpc.org/specification#response_object>
            #[derive(Deserialize)]
            struct RequestId {
                id: Id,
            }

            let id =
                serde_json::from_slice::<RequestId>(json).map_or(Id::Null, |request| request.id);

            return Some(JsonRpcResponse::invalid_request(id));
        }
    };

    // A method that is only for non-restricted RPC is answered as if it did
    // not exist, so a restricted server does not leak which methods exist.
    //
    // INVARIANT:
    // The RPC handler functions in `cuprated` depend on this check existing,
    // the functions themselves do not check if they are being called
    // from an (un)restricted context. This must be here or all methods will
    // be allowed to be called freely, including by omitting `id`.
    let method = match request.body {
        // The error when a restricted JSON-RPC method is called as per:
        //
        // - <https://github.com/monero-project/monero/blob/893916ad091a92e765ce3241b94e706ad012b62a/contrib/epee/include/net/http_server_handlers_map2.h#L244-L252>
        // - <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L188>
        JsonRpcMethod::Known(body) if handler.is_restricted() && body.is_restricted() => {
            Err(ErrorObject::method_not_found())
        }
        JsonRpcMethod::InvalidParams { restricted: true } if handler.is_restricted() => {
            Err(ErrorObject::method_not_found())
        }
        JsonRpcMethod::Known(body) => Ok(body),
        JsonRpcMethod::Unknown => Err(ErrorObject::method_not_found()),
        JsonRpcMethod::InvalidParams { .. } => Err(ErrorObject::invalid_params()),
    };

    // JSON-RPC 2.0 rule:
    // <https://www.jsonrpc.org/specification#notification>
    //
    // JSON-RPC notifications (requests without `id`) must not be responded to,
    // although their side-effects must remain, so the request is still sent.
    match request.id {
        None => {
            if let Ok(body) = method {
                drop(handler.oneshot(body).await);
            }
            None
        }
        Some(id) => Some(match method {
            Err(error) => JsonRpcResponse::err(id, error),
            Ok(body) => match handler.oneshot(body).await {
                Ok(response) => JsonRpcResponse::ok(id, response),
                Err(_) => JsonRpcResponse::internal_error(id),
            },
        }),
    }
}

/// Whether the request has a JSON media type.
fn json_content_type(headers: &HeaderMap) -> bool {
    let Some(value) = headers.get(CONTENT_TYPE) else {
        return false;
    };

    let value = value.as_bytes();

    // Nearly every client sends exactly this.
    if value == b"application/json" {
        return true;
    }

    // Ignore any `;` parameters, e.g. `application/json; charset=utf-8`.
    let media_type = match value.iter().position(|&b| b == b';') {
        Some(i) => &value[..i],
        None => value,
    }
    .trim_ascii();

    let Some((prefix, subtype)) = media_type.split_at_checked("application/".len()) else {
        return false;
    };

    prefix.eq_ignore_ascii_case(b"application/")
        && !subtype.contains(&b'/')
        && (subtype.eq_ignore_ascii_case(b"json")
            || subtype.len() > "+json".len()
                && subtype[subtype.len() - "+json".len()..].eq_ignore_ascii_case(b"+json"))
}

/// Append a response to an existing JSON buffer.
fn write_response_json(response: JsonRpcResponse<RpcResponse>, json: &mut Vec<u8>) {
    let start = json.len();
    let result = match &response.payload {
        Ok(RpcResponse::GetTxpoolBacklog(body)) => {
            serialize_txpool_backlog_response(json, &response.id, body)
        }
        _ => serde_json::to_writer(&mut *json, &response),
    };

    if result.is_err() {
        json.truncate(start);
        serde_json::to_writer(
            json,
            &JsonRpcResponse::<RpcResponse>::internal_error(response.id),
        )
        .expect("an error response always serializes");
    }
}

/// Serialize a JSON-RPC response into an HTTP response.
fn json_rpc_response(response: JsonRpcResponse<RpcResponse>) -> Response {
    let mut json = Vec::new();
    write_response_json(response, &mut json);

    Response::builder()
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json))
        .expect("valid response builder")
}

// TODO: remove the code below once this: https://github.com/monero-project/monero/issues/9422 is resolved.

/// Serialize the backlog's POD container as a binary JSON string.
fn serialize_txpool_backlog_response(
    json: &mut Vec<u8>,
    id: &Id,
    response: &GetTxpoolBacklogResponse,
) -> serde_json::Result<()> {
    json.extend_from_slice(
        br#"{
            "jsonrpc":"2.0",
            "id":"#,
    );
    serde_json::to_writer(&mut *json, id)?;
    json.extend_from_slice(
        br#",
            "result":"#,
    );
    serde_json::to_writer(&mut *json, &response.base)?;

    if !response.backlog.is_empty() {
        // Reopen `result` to append the field that serde cannot represent.
        let Some(b'}') = json.pop() else {
            unreachable!("response base must serialize as a JSON object");
        };
        json.extend_from_slice(br#","backlog":"#);
        write_binary_string(json, &txpool_backlog_blob(response));
        json.push(b'}');
    }

    json.push(b'}');
    Ok(())
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
