# JSON-RPC 2.0
Cuprate has a standalone crate that implements the [JSON-RPC 2.0](https://www.jsonrpc.org/specification) specification,  [`cuprate-json-rpc`](https://doc.cuprate.org/cuprate_json_rpc). The RPC methods at the `/json_rpc` endpoint use this crate's types, functions, and (de)serialization.

There is nothing too special about Cuprate's implementation.
Any small notes and differences are noted in the crate documentation.

As such, there is not much to document here, instead, consider reading the very
brief JSON-RPC 2.0 specification, and the `cuprate-json-rpc` crate documentation.

> TODO: document `method/params` vs flattened `base` when figured out.
