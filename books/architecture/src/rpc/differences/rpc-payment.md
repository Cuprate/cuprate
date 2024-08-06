# RPC payment
The RPC payment system in `monerod` is a [pseudo-deprecated](https://github.com/monero-project/monero/issues/8722)
system that allows node operators to be compensated for RPC usage.

Although this system is pseudo-deprecated, `monerod` still generates related fields in responses. [Cuprate follows this behavior](https://doc.cuprate.org/cuprate_rpc_types/base/struct.AccessResponseBase.html).

However, the [associated endpoints](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L182-L187) and [actual functionality](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L260-L265) are not supported by Cuprate. The associated endpoints will return an error upon invocation.

> TODO: decide on behavior and document <https://github.com/Cuprate/cuprate/pull/233#discussion_r1700870051>.