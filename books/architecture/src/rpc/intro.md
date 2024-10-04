# RPC
`monerod`'s daemon RPC has three kinds of RPC calls:
1. [JSON-RPC 2.0](https://www.jsonrpc.org/specification) methods, called at the `/json_rpc` endpoint
1. JSON (but not JSON-RPC 2.0) methods called at their own endpoints, e.g. [`/get_height`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_height)
1. Binary ([epee](../../formats-protocols-types/epee.html)) RPC methods called at their own endpoints ending in `.bin`, e.g. [`/get_blocks.bin`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin)

Cuprate's RPC aims to mirror `monerod`'s as much as it practically can.

This includes, but is not limited to:
- Using the same endpoints
- Receiving the same request data
- Sending the same response data
- Responding with the same HTTP status codes
- Following internal behavior (e.g. [`/pop_blocks`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#pop_blocks))

Not all `monerod` behavior can always be followed, however.

Some are not followed on purpose, some cannot be followed due to technical limitations, and some cannot be due to the behavior being `monerod` specific such as the [`/set_log_categories`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#set_log_categories) endpoint which uses `monerod`'s logging categories.

Both subtle and large differences between Cuprate's RPC and `monerod`'s RPC are documented in the [Differences with `monerod`](differences/intro.md) section.

## Main RPC components
The main components that make up Cuprate's RPC are noted below, alongside the equivalent `monerod` code and other notes.

| Cuprate crate | `monerod` (rough) equivalent | Purpose | Notes |
|---------------|------------------------------|---------|-------|
| [`cuprate-json-rpc`](https://doc.cuprate.org/cuprate_json_rpc) | [`jsonrpc_structs.h`](https://github.com/monero-project/monero/blob/caa62bc9ea1c5f2ffe3ffa440ad230e1de509bfd/contrib/epee/include/net/jsonrpc_structs.h), [`http_server_handlers_map2.h`](https://github.com/monero-project/monero/blob/caa62bc9ea1c5f2ffe3ffa440ad230e1de509bfd/contrib/epee/include/net/http_server_handlers_map2.h) | JSON-RPC 2.0 implementation | `monerod`'s JSON-RPC 2.0 handling is spread across a few files. The first defines some data structures, the second contains macros that (essentially) implement JSON-RPC 2.0.
| [`cuprate-rpc-types`](https://doc.cuprate.org/cuprate_rpc_types) | [`core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/caa62bc9ea1c5f2ffe3ffa440ad230e1de509bfd/src/rpc/core_rpc_server_commands_defs.h) | RPC request/response type definitions and (de)serialization | |
| [`cuprate-rpc-interface`](https://doc.cuprate.org/cuprate_rpc_interface) | [`core_rpc_server.h`](https://github.com/monero-project/monero/blob/caa62bc9ea1c5f2ffe3ffa440ad230e1de509bfd/src/rpc/core_rpc_server.h) | RPC interface, routing, endpoints | |
| [`cuprate-rpc-handler`](https://doc.cuprate.org/cuprate_rpc_handler) | [`core_rpc_server.cpp`](https://github.com/monero-project/monero/blob/caa62bc9ea1c5f2ffe3ffa440ad230e1de509bfd/src/rpc/core_rpc_server.cpp) | RPC request/response handling | These are the "inner handler" functions that turn requests into responses |