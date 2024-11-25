Monero RPC types.

# What
This crate ports the types used in Monero's RPC interface, including:
- JSON types
- Binary (epee) types
- Mixed types
- Other commonly used RPC types

It also includes some traits for these types.

# Modules
This crate's types are split in the following manner:

| Module | Purpose |
|--------|---------|
| The root module | Miscellaneous items, e.g. constants.
| [`json`] | Contains JSON request/response (some mixed with binary) that all share the common `/json_rpc` endpoint. |
| [`bin`] | Contains request/response types that are expected to be fully in binary (`cuprate_epee_encoding`) in `monerod` and `cuprated`'s RPC interface. These are called at a custom endpoint instead of `/json_rpc`, e.g. `/get_blocks.bin`. |
| [`other`] | Contains request/response types that are JSON, but aren't called at `/json_rpc` (e.g. [`crate::other::GetHeightRequest`]). |
| [`misc`] | Contains miscellaneous types, e.g. [`crate::misc::Status`]. Many of types here are found and used in request/response types, for example, [`crate::misc::BlockHeader`] is used in [`crate::json::GetLastBlockHeaderResponse`]. |
| [`base`] | Contains base types flattened into many request/response types.

Each type in `{json,bin,other}` come in pairs and have identical names, but are suffixed with either `Request` or `Response`. e.g. [`GetBlockCountRequest`](crate::json::GetBlockCountRequest) & [`GetBlockCountResponse`](crate::json::GetBlockCountResponse).

# Documentation
The documentation for types within `{json,bin,other}` are omitted, as they can be found in [Monero's RPC documentation](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html).

However, each type will document:
- **Definition**: the exact type definition location in `monerod`
- **Documentation**: the Monero RPC documentation link
- **Request/response**: the other side of this type, either the request or response

# Naming
The naming for types within `{json,bin,other}` follow the following scheme:
1. Convert the endpoint or method name into `UpperCamelCase`
1. Remove any suffix extension
1. Add `Request/Response` suffix

For example:

| Endpoint/method | Crate location and name |
|-----------------|-------------------------|
| [`get_block_count`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_block_count) | [`json::GetBlockCountRequest`] & [`json::GetBlockCountResponse`]
| [`/get_blocks.bin`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blockbin) | [`bin::GetBlocksRequest`] & [`bin::GetBlocksResponse`]
| [`/get_height`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_height) | [`other::GetHeightRequest`] & [`other::GetHeightResponse`]

# Deprecated types
TODO: update after finalizing <https://github.com/monero-project/monero/issues/9422>.

- [`crate::json::GetTransactionPoolBacklogV2Response`]
- [`crate::json::GetOutputDistributionV2Response`]

# Optimized types
TODO: updated after deciding compatability <-> optimization tradeoff.

- Fixed byte containers

<!--

Some fields within requests/responses are containers, but fixed in size.

For example, [`crate::json::GetBlockTemplateResponse::prev_hash`] is always a 32-byte hash.

In these cases, stack allocated types like `cuprate_fixed_bytes::StrArray`
will be used instead of a more typical [`String`] for optimization reasons.

-->

# (De)serialization invariants
Due to how types are defined in this library internally (all through a single macro),
most types implement both `serde` and `epee`.

However, some of the types will panic with [`unimplemented`]
or will otherwise have undefined implementation in the incorrect context.

In other words:
- The epee (de)serialization of [`json`] & [`other`] types should **not** be relied upon
- The JSON (de)serialization of [`bin`] types should **not** be relied upon

The invariants that can be relied upon:
- Types in [`json`] & [`other`] will implement `serde` correctly
- Types in [`bin`] will implement `epee` correctly
- Misc types will implement `serde/epee` correctly as needed

# Requests and responses
For `enum`s that encapsulate all request/response types, see:
- [`crate::json::JsonRpcRequest`] & [`crate::json::JsonRpcResponse`]
- [`crate::bin::BinRequest`] & [`crate::bin::BinResponse`]
- [`crate::other::OtherRequest`] & [`crate::other::OtherResponse`]

# Feature flags
List of feature flags for `cuprate-rpc-types`.

All are enabled by default.

| Feature flag | Does what |
|--------------|-----------|
| `serde`      | Implements `serde` on all types
| `epee`       | Implements `cuprate_epee_encoding` on all types