# `cuprate-rpc-interface`
This crate provides Cuprate's RPC _interface_.

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
it implements the regular RPC server modeled after `monerod`.

# Purpose
`cuprate-rpc-interface` is built on-top of [`axum`],
which is the crate _actually_ handling everything.

This crate simply handles:
- Registering endpoint routes (e.g. `/get_block.bin`)
- Handler function signatures
- (De)serialization of data (JSON-RPC methods/params, binary, JSON)

The actual server details are all handled by the [`axum`] and [`tower`] ecosystem, i.e.
the whole purpose of this crate is to:
1. Implement a [`RpcHandler`]
2. Use it with [`create_router`] to generate an
   [`axum::Router`] with all Monero RPC routes/types set
4. Do whatever with it

# The [`RpcHandler`]
This is your [`tower::Service`] that converts [`RpcRequest`]s into [`RpcResponse`]s,
i.e. the "inner handler".

Said concretely, [`RpcHandler`] is a `tower::Service` where the associated types are from this crate:
- [`RpcRequest`]
- [`RpcResponse`]
- [`RpcError`]

The [`RpcHandler`] must also hold some state that is required
for RPC server operation.

The only state currently need is [`RpcHandler::restricted`], which determines if an RPC
server is restricted or not, and thus, if some endpoints/methods are allowed or not.

# Example
Example usage of this crate + starting an RPC server.

This uses `RpcHandlerDummy` as the handler; it responds with the
correct response, but always with a default value.

```rust
use std::sync::Arc;

use axum::{extract::Request, response::Response};
use tokio::{net::TcpListener, sync::Barrier};

use cuprate_rpc_types::other::{OtherRequest, OtherResponse};
use cuprate_rpc_interface::{create_router, RpcHandlerDummy, RpcRequest};

#[tokio::main]
async fn main() {
    // Create the router.
    let state = RpcHandlerDummy { restricted: false };
    let router = create_router().with_state(state);

    // Start a server.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();

    // Run the server with `axum`.
    let barrier = Arc::new(Barrier::new(2));
    let barrier2 = barrier.clone();
    tokio::task::spawn(async move {
        barrier2.wait();
        axum::serve(listener, router).await.unwrap();
    });
    barrier.wait();

    // Send a request.
    // TODO: endpoints without inputs shouldn't error without input.
    let url = format!("http://127.0.0.1:{port}/get_height");
    let request = OtherRequest::GetHeight(Default::default());
    let body: OtherResponse = ureq::get(&url)
        .set("Content-Type", "application/json")
        .send_json(request)
        .unwrap()
        .into_json()
        .unwrap();

    // Assert the response is as expected.
    // We just made an RPC call :)
    let expected = OtherResponse::GetHeight(Default::default());
    assert_eq!(body, expected);
}
```

# Feature flags
List of feature flags for `cuprate-rpc-interface`.

All are enabled by default.

| Feature flag | Does what |
|--------------|-----------|
| `serde`      | Enables serde on applicable types
| `dummy`      | Enables the `RpcHandlerDummy` type