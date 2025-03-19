# The interface
> This section is short as [`cuprate-rpc-interface`](https://doc.cuprate.org/cuprate_rpc_interface) contains detailed documentation.

The RPC interface, which includes:

- Endpoint routing (`/json_rpc`, `/get_blocks.bin`, etc)
- Route function signatures (`async fn json_rpc(...) -> Response`)
- Type (de)serialization
- Any miscellaneous handling (denying `restricted` RPC calls)

is handled by the [`cuprate-rpc-interface`](https://doc.cuprate.org/cuprate_rpc_interface) crate.

Essentially, this crate provides the API for the RPC.

`cuprate-rpc-interface` is built on-top of [`axum`](https://docs.rs/axum) and [`tower`](https://docs.rs/tower),
which are the crates doing the bulk majority of the work.

## Request -> Response
The functions that map requests to responses are not implemented by `cuprate-rpc-interface` itself, they must be provided by the user, i.e. it can be _customized_.

In Rust terms, this crate provides you with:
```rust
async fn json_rpc(
	state: State,
	request: Request,
) -> Response {
	/* your handler here */
}
```
and you provide the function body.

The main handler crate is `cuprated` itself, it implements the standard RPC behavior, i.e. it mostly mirrors `monerod`.

Although, it's worth noting that other implementations are possible, such as an RPC handler that caches blocks,
or an RPC handler that only accepts certain endpoints, or any combination.