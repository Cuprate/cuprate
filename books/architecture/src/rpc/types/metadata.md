# Metadata
[`cuprate-rpc-types`](https://doc.cuprate.org/cuprate_rpc_types) also provides
some `trait`s to access some metadata surrounding RPC data types.

For example, [`trait RpcCall`](https://doc.cuprate.org/cuprate_rpc_types/trait.RpcCall.html)
allows accessing whether an RPC request is [`restricted`](https://doc.cuprate.org/cuprate_rpc_types/trait.RpcCall.html#associatedconstant.IS_RESTRICTED) or not.

`monerod` has a boolean permission system. RPC calls can be restricted or not.
If an RPC call is restricted, it will only be allowed on un-restricted RPC servers (`18081`).
If an RPC call is _not_ restricted, it will be allowed on all RPC server types (`18081` & `18089`).

This metadata is used in crates that build upon `cuprate-rpc-types`, e.g.
to know if an RPC call should be allowed through or not.