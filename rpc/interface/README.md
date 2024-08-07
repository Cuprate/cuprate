# `cuprate-rpc-interface`
This crate provides Cuprate's RPC _interface_.

This crate is _not_ a standalone RPC server, it is just the interface.

```text
            cuprate-rpc-interface provides these parts
                            │         │
┌───────────────────────────┤         ├───────────────────┐
▼                           ▼         ▼                   ▼
CLIENT ─► ROUTE ─► REQUEST ─► HANDLER ─► RESPONSE ─► CLIENT
                             ▲       ▲
                             └───┬───┘
                                 │
                      You provide this part
```

Everything coming _in_ from a client is handled by this crate.

This is where your [`RpcHandler`] turns this [`RpcRequest`] into a [`RpcResponse`].

You hand this `Response` back to `cuprate-rpc-interface` and it will take care of sending it back to the client.

The main handler used by Cuprate is implemented in the `cuprate-rpc-handler` crate;
it implements the standard RPC handlers modeled after `monerod`.

# Purpose
`cuprate-rpc-interface` is built on-top of [`axum`],
which is the crate _actually_ handling everything.

This crate simply handles:
- Registering endpoint routes (e.g. `/get_block.bin`)
- Defining handler function signatures
- (De)serialization of requests/responses (JSON-RPC, binary, JSON)

The actual server details are all handled by the [`axum`] and [`tower`] ecosystem.

The proper usage of this crate is to:
1. Implement a [`RpcHandler`]
2. Use it with [`RouterBuilder`] to generate an
   [`axum::Router`] with all Monero RPC routes set
3. Do whatever with it

# The [`RpcHandler`]
This is your [`tower::Service`] that converts [`RpcRequest`]s into [`RpcResponse`]s,
i.e. the "inner handler".

Said concretely, `RpcHandler` is a `tower::Service` where the associated types are from this crate:
- [`RpcRequest`]
- [`RpcResponse`]
- [`RpcError`]

`RpcHandler`'s [`Future`](std::future::Future) is generic, _although_,
it must output `Result<RpcResponse, RpcError>`.

The `RpcHandler` must also hold some state that is required
for RPC server operation.

The only state currently needed is [`RpcHandler::restricted`], which determines if an RPC
server is restricted or not, and thus, if some endpoints/methods are allowed or not.

# Unknown endpoint behavior
TODO: decide what this crate should return (per different endpoint)
when a request is received to an unknown endpoint, including HTTP stuff, e.g. status code.

# Unknown JSON-RPC method behavior
TODO: decide what this crate returns when a `/json_rpc`
request is received with an unknown method, including HTTP stuff, e.g. status code.

# Example
Example usage of this crate + starting an RPC server.

This uses `RpcHandlerDummy` as the handler; it always responds with the
correct response type, but set to a default value regardless of the request.

```rust
use std::sync::Arc;

use tokio::{net::TcpListener, sync::Barrier};

use cuprate_json_rpc::{Request, Response, Id};
use cuprate_rpc_types::{
    json::{JsonRpcRequest, JsonRpcResponse, GetBlockCountResponse},
    other::{OtherRequest, OtherResponse},
};
use cuprate_rpc_interface::{RouterBuilder, RpcHandlerDummy, RpcRequest};

// Send a `/get_height` request. This endpoint has no inputs.
async fn get_height(port: u16) -> OtherResponse {
    let url = format!("http://127.0.0.1:{port}/get_height");
    ureq::get(&url)
        .set("Content-Type", "application/json")
        .call()
        .unwrap()
        .into_json()
        .unwrap()
}

// Send a JSON-RPC request with the `get_block_count` method.
//
// The returned [`String`] is JSON.
async fn get_block_count(port: u16) -> String {
    let url = format!("http://127.0.0.1:{port}/json_rpc");
    let method = JsonRpcRequest::GetBlockCount(Default::default());
    let request = Request::new(method);
    ureq::get(&url)
        .set("Content-Type", "application/json")
        .send_json(request)
        .unwrap()
        .into_string()
        .unwrap()
}

#[tokio::main]
async fn main() {
    // Start a local RPC server.
    let port = {
        // Create the router.
        let state = RpcHandlerDummy { restricted: false };
        let router = RouterBuilder::new().all().build().with_state(state);

        // Start a server.
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();

        // Run the server with `axum`.
        tokio::task::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        port
    };

    // Assert the response is the default.
    let response = get_height(port).await;
    let expected = OtherResponse::GetHeight(Default::default());
    assert_eq!(response, expected);

    // Assert the response JSON is correct.
    let response = get_block_count(port).await;
    let expected = r#"{"jsonrpc":"2.0","id":null,"result":{"status":"OK","untrusted":false,"count":0}}"#;
    assert_eq!(response, expected);

    // Assert that (de)serialization works.
    let expected = Response::ok(Id::Null, Default::default());
    let response: Response<GetBlockCountResponse> = serde_json::from_str(&response).unwrap();
    assert_eq!(response, expected);
}
```

# Feature flags
List of feature flags for `cuprate-rpc-interface`.

All are enabled by default.

| Feature flag | Does what |
|--------------|-----------|
| `serde`      | Enables serde on applicable types
| `dummy`      | Enables the `RpcHandlerDummy` type