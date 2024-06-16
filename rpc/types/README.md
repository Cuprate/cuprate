Monero RPC types.

# What
This crate ports the types used in Monero's RPC interface, including:
- JSON types
- Binary (epee) types
- Mixed types
- Other commonly used RPC types

# Modules
This crate's types are split in the following manner:

This crate has 4 modules:
- The root module; `cuprate_rpc_types`
- [`json`] module; JSON types from the `/json_rpc` endpoint
- [`bin`] module; Binary types from the binary endpoints
- [`other`] module; Misc JSON types from other endpoints

Miscellaneous types are found in the root module, e.g. [`crate::Status`].

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
| [`/get_blocks.bin`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blockbin) | `bin::GetBlocksRequest` & `bin::GetBlocksResponse`
| [`/get_height`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_height) | `other::GetHeightRequest` & `other::GetHeightResponse`

TODO: fix doc links when types are ready.

# Mixed types
Note that some types within [`other`] mix JSON & binary together, i.e.,
the message overall is JSON, however some fields contain binary
values, for example:

```json
{
  "string": "",
  "float": 30.0,
  "integer": 30,
  "binary": /* serialized binary */
}
```

TODO: list the specific types.