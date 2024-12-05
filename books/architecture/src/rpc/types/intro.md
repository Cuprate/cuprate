# The types
Cuprate has a crate that defines all the types related to RPC: [`cuprate-rpc-types`](https://doc.cuprate.org/cuprate_rpc_types).

The main purpose of this crate is to port the types used in `monerod`'s RPC and to re-implement
(de)serialization for those types, whether that be JSON, `epee`, or a custom mix.

The bulk majority of these types are [request & response types](macro.md), i.e. the inputs
Cuprate's RPC is expecting from users, and the output it will respond with.

## Example
To showcase an example of the kinds of types defined in this crate, here is a request type:
```rust
#[serde(transparent)]
#[repr(transparent)]
struct OnGetBlockHashRequest {
	block_height: [u64; 1],
}
```
This is the input (`params`) expected in the [`on_get_block_hash`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#on_get_block_hash) method.

As seen above, the type itself encodes some properties, such as being (de)serialized [transparently](https://serde.rs/container-attrs.html#transparent), and the input being an array with 1 length, rather than a single `u64`. [This is to match the behavior of `monerod`](https://github.com/monero-project/monero/blob/caa62bc9ea1c5f2ffe3ffa440ad230e1de509bfd/src/rpc/core_rpc_server.cpp#L1826).

An example JSON form of this type would be:
```json
{
  "jsonrpc": "2.0",
  "id": "0",
  "method": "on_get_block_hash",
  "params": [912345] // <- This can (de)serialize as a `OnGetBlockHashRequest`
}
```