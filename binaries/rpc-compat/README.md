# `cuprate-rpc-compat`
This crate provides tools for testing compatibility between `monerod` and `cuprated`'s daemon RPC API, specifically:
- Request/response type correctness (both have same schema)
- Value correctness (both output the same values)

There are cases where type/value correctness cannot be upheld by `cuprated`,
this crate provides tools for these cases as well.

# Harness
TODO

# Input permutations
TODO

# Leniency on correctness
TODO

# Configuring the RPC servers
TODO

# Example
TODO

```rust
// use std::sync::Arc;

// use tokio::{net::TcpListener, sync::Barrier};

// use cuprate_json_rpc::{Request, Response, Id};
// use cuprate_rpc_types::{
//     json::{JsonRpcRequest, JsonRpcResponse, GetBlockCountResponse},
//     other::{OtherRequest, OtherResponse},
// };
// use cuprate_rpc_interface::{RouterBuilder, RpcHandlerDummy};

// // Send a `/get_height` request. This endpoint has no inputs.
// async fn get_height(port: u16) -> OtherResponse {
//     let url = format!("http://127.0.0.1:{port}/get_height");
//     ureq::get(&url)
//         .set("Content-Type", "application/json")
//         .call()
//         .unwrap()
//         .into_json()
//         .unwrap()
// }

// // Send a JSON-RPC request with the `get_block_count` method.
// //
// // The returned [`String`] is JSON.
// async fn get_block_count(port: u16) -> String {
//     let url = format!("http://127.0.0.1:{port}/json_rpc");
//     let method = JsonRpcRequest::GetBlockCount(Default::default());
//     let request = Request::new(method);
//     ureq::get(&url)
//         .set("Content-Type", "application/json")
//         .send_json(request)
//         .unwrap()
//         .into_string()
//         .unwrap()
// }

// #[tokio::main]
// async fn main() {
//     // Start a local RPC server.
//     let port = {
//         // Create the router.
//         let state = RpcHandlerDummy { restricted: false };
//         let router = RouterBuilder::new().all().build().with_state(state);

//         // Start a server.
//         let listener = TcpListener::bind("127.0.0.1:0")
//             .await
//             .unwrap();
//         let port = listener.local_addr().unwrap().port();

//         // Run the server with `axum`.
//         tokio::task::spawn(async move {
//             axum::serve(listener, router).await.unwrap();
//         });

//         port
//     };

//     // Assert the response is the default.
//     let response = get_height(port).await;
//     let expected = OtherResponse::GetHeight(Default::default());
//     assert_eq!(response, expected);

//     // Assert the response JSON is correct.
//     let response = get_block_count(port).await;
//     let expected = r#"{"jsonrpc":"2.0","id":null,"result":{"status":"OK","untrusted":false,"count":0}}"#;
//     assert_eq!(response, expected);

//     // Assert that (de)serialization works.
//     let expected = Response::ok(Id::Null, Default::default());
//     let response: Response<GetBlockCountResponse> = serde_json::from_str(&response).unwrap();
//     assert_eq!(response, expected);
// }
```

# Feature flags
List of feature flags for `cuprate-rpc-compat`.

All are enabled by default.

| Feature flag | Does what |
|--------------|-----------|
| TODO         | TODO