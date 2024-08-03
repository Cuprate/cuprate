# Base RPC types
There exists a few "base" types that many types are built on-top of in `monerod`.
These are also implemented in [this crate](https://doc.cuprate.org/cuprate_rpc_types/base/index.html).

For example, many requests include these 2 fields:
```json
{
  "status": "OK",
  "untrusted": false,
}
```
This is [`rpc_response_base`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L101-L112) in `monerod`, and [`ResponseBase`](https://doc.cuprate.org/cuprate_rpc_types/base/struct.ResponseBase.html) in Cuprate.

These types are [flattened](https://serde.rs/field-attrs.html#flatten) into types, i.e. the fields
from these base types are injected into the given type. For example, [`get_block_count`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_block_count)'s response type is defined like such in Cuprate:
```rust
struct GetBlockCountResponse {
	// The fields of this `base` type are directly
	// injected into `GetBlockCountResponse` during
	// (de)serialization.
	//
	// I.e. it is as if this `base` field were actually these 2 fields:
	// status: Status,
	// untrusted: bool,
    base: ResponseBase,
	count: u64,
}
```
The JSON output of this type would look something like:
```json
{
  "status": "OK",
  "untrusted": "false",
  "count": 993163
}
```

## RPC payment
`monerod` also contains RPC base types for the [RPC payment](https://doc.cuprate.org/cuprate_rpc_types/base/struct.AccessResponseBase.html) system. Although the RPC payment system [is](https://github.com/monero-project/monero/issues/8722) [pseudo](https://github.com/monero-project/monero/pull/8724) [deprecated](https://github.com/monero-project/monero/pull/8843), `monerod` still generates these fields in responses, and thus, [so does Cuprate](https://doc.cuprate.org/cuprate_rpc_types/base/struct.AccessResponseBase.html).