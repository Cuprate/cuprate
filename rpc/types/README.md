Monero RPC types.

# What
This crate ports the types used in Monero's RPC interface, including:
- JSON types
- Binary (epee) types
- Mixed types
- Other commonly used RPC types

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
- Convert the endpoint or method name into `UpperCamelCase`
- Remove any suffix extension

For example:

| Endpoint/method | Crate location and name |
|-----------------|-------------------------|
| [`get_block_count`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_block_count) | [`json::GetBlockCountRequest`] & [`json::GetBlockCountResponse`]
| [`/get_blocks.bin`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blockbin) | [`bin::GetBlocksRequest`] & [`bin::GetBlocksResponse`]
| [`/get_height`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_height) | `other::GetHeightRequest` & `other::GetHeightResponse`

TODO: fix doc links when types are ready.

# Mixed types
Note that some types mix JSON & binary together, i.e., the message overall is JSON,
however some fields contain binary values inside JSON strings, for example:

```json
{
  "string": "",
  "float": 30.0,
  "integer": 30,
  "binary": "<serialized binary>"
}
```

`binary` here is (de)serialized as a normal [`String`]. In order to be clear on which fields contain binary data, the struct fields that have them will use [`crate::BinaryString`] instead of [`String`].

- TODO: list the specific types.
- TODO: we need to figure out a type that (de)serializes correctly, `String` errors with `serde_json`

# Feature flags
List of feature flags for `cuprate-rpc-types`.

All are enabled by default.

| Feature flag | Does what |
|--------------|-----------|
| `json`       | Enables the `crate::json` module
| `bin`        | Enables the `crate::bin` module
| `other`      | Enables the `crate::other` module
| `serde`      | Implements `serde` on all types
| `epee`       | Implements `cuprate_epee_encoding` on all types