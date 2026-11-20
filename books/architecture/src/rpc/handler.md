# The handler
The handlers (functions that map requests into responses) are / can be generic with `cuprate-rpc-interface`.

`cuprated` itself implements the standard RPC handlers modeled after `monerod`, see here:

- [JSON-RPC](https://github.com/Cuprate/cuprate/tree/main/binaries/cuprated/src/rpc/handlers/json_rpc.rs)
- [Binary](https://github.com/Cuprate/cuprate/tree/main/binaries/cuprated/src/rpc/handlers/bin.rs)
- [Other JSON](https://github.com/Cuprate/cuprate/tree/main/binaries/cuprated/src/rpc/handlers/other_json.rs)

The main job of the handler function is to do what is necessary to map the request type into the response type. This often requires calling deeper into other parts of the `cuprated` such as the blockchain service. After the necessary data is collected, the response is created and returned.

In general, the handler functions are 1-1 with RPC calls themselves, e.g. `/get_height` is handled by [`get_height()`](https://github.com/Cuprate/cuprate/blob/e6efdbb437948a3c38938dcbb75f0c37d7e1e9d0/binaries/cuprated/src/rpc/handlers/other_json.rs#L110-L123), although there are some shared internal functions such as [`/get_outs` and `/get_outs.bin`](https://github.com/Cuprate/cuprate/blob/e6efdbb437948a3c38938dcbb75f0c37d7e1e9d0/binaries/cuprated/src/rpc/handlers/shared.rs#L35-L75).