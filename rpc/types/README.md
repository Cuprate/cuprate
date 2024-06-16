Monero RPC types.

# What
This crate ports the types used in Monero's RPC interface, including:
- JSON types
- Binary (epee) types
- Mixed types
- Other commonly used RPC types

# Modules
This crate's types are split in the following manner:

1. This crate has 3 modules:
    - The root module (`cuprate_rpc_types`)
    - [`req`] (request types)
    - [`resp`] (response types)
1. Miscellaneous types are found in the root module, e.g. [`Status`]
1. The `req` and `resp` modules perfectly mirror each-other, and are split into 3 modules:
    - `json` (JSON types from the `/json_rpc` endpoint)
    - `bin` (Binary types from the binary endpoints)
    - `other` (Misc JSON types from other endpoints)
1. Each type in `req` has a corresponding type in `resp` and vice-versa with an identical name, e.g. [`req::json::GetBlockCount`] and [`resp::json::GetBlockCount`]

# Documentation
The documentation for types within [`req`] and [`resp`] are omitted,
as they can be found in [Monero's RPC documentation](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#on_get_block_hash).

However, each type will document:
- The exact type definition location in `monerod`
- The Monero RPC documentation link

# Naming
The naming for types within [`req`] and [`resp`] follow the following scheme:
- Convert the endpoint or method name into `UpperCamelCase`
- Remove any suffix extension

For example:

| Endpoint/method | Crate location and name |
|-----------------|-------------------------|
| [`get_block_count`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_block_count) | [`req::json::GetBlockCount`] & [`resp::json::GetBlockCount`]
| [`/get_blocks.bin`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blockbin) | `req::bin::GetBlocks` & `resp::bin::GetBlocks`
| [`/get_height`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_height) | `req::other::GetHeight` & `resp::other::GetHeight`

TODO: fix doc links when types are ready.

# Mixed types
Note that some types within [`resp::other`] mix JSON & binary, i.e.,
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