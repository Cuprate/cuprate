# The interface
> This section is short as [`cuprate-rpc-interface`](https://doc.cuprate.org/cuprate_rpc_interface) contains detailed documentation.

The RPC interface, which includes:

- Endpoint routing (`/json_rpc`, `/get_blocks.bin`, etc)
- Route function signatures (`async fn json_rpc(...) -> Response`)
- Type (de)serialization
- Any miscellaneous handling (denying `restricted` RPC calls)

is handled by the [`cuprate-rpc-interface`](https://doc.cuprate.org/cuprate_rpc_interface) crate.

```text
            cuprate-rpc-interface provides these parts
                            │         │
┌───────────────────────────┤         ├───────────────────┐
▼                           ▼         ▼                   ▼
CLIENT ─► ROUTE ─► REQUEST ─► HANDLER ─► RESPONSE ─► CLIENT
                             ▲       ▲
                             └───┬───┘
                                 │
                      Users provide this part
```

Essentially, this crate provides the API for the RPC.

The functions that map requests to responses must be provided by the user, i.e. it can be _customized_.

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

The main handler crate is [`cuprate-rpc-handler`](https://doc.cuprate.org/cuprate_rpc_handler).
This crate implements the standard RPC behavior, i.e. it mostly mirrors `monerod`.

Although, it's worth noting that other implementations are possible, such as an RPC handler that caches blocks,
or an RPC handler that only accepts certain endpoints, or any combination.

## RpcHandler
This is the main trait that abstracts over inner handlers.

This trait represents and object that also implements [`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) that converts requests into responses.

See [`cuprate_rpc_interface::RpcHandler`](https://doc.cuprate.org/cuprate_rpc_interface/trait.RpcHandler)
for detailed documentation.

## Router
The main purpose of `cuprate-rpc-interface` is to:
1. Implement a `RpcHandler`
1. Use it with [`RouterBuilder`](https://doc.cuprate.org/cuprate_rpc_interface/struct.RouterBuilder) to generate an
   [`axum::Router`](https://docs.rs/axum/latest/axum/struct.Router.html) with all Monero RPC routes set
1. Do whatever with it

`cuprate-rpc-interface` is built on-top of [`axum`](https://docs.rs/axum),
which is the crate _actually_ handling everything.

This crate simply handles:
- Registering endpoint routes (e.g. `/get_block.bin`)
- Defining handler function signatures

The actual server details are all handled by the `axum` and `tower` ecosystems.