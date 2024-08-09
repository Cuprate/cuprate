# (De)serialization
A crucial responsibility of [`cuprate-rpc-types`](https://doc.cuprate.org/cuprate_rpc_types)
is to provide the _correct_ (de)serialization of types.

The input/output of Cuprate's RPC should match `monerod` (as much as practically possible).

A simple example of this is that [`/get_height`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_height)
should respond with the exact same data for both `monerod` and Cuprate:
```json
{
  "hash": "7e23a28cfa6df925d5b63940baf60b83c0cbb65da95f49b19e7cf0ce7dd709ce",
  "height": 2287217,
  "status": "OK",
  "untrusted": false
}
```
Behavior would be considered incompatible if any of the following were true:
- Fields are missing
- Extra fields exist
- Field types are incorrect (`string` instead of `number`, etc)

## JSON
(De)serialization for JSON is implemented using [`serde`](https://docs.rs/serde) and [`serde_json`](https://docs.rs/serde_json).

[`cuprate-rpc-interface`](https://doc.cuprate.org/cuprate_rpc_interface) (the main crate responsible
for the actual output) uses `serde_json` for JSON formatting. It is _mostly_ the same formatting as `monerod`, [although there are slight differences](../differences/json-formatting.md).

Technically, the formatting of the JSON output is not handled by `cuprate-rpc-types`, users are free to choose whatever formatting they desire.

## Epee
(De)serialization for the [epee binary format](../../formats-protocols-types/epee.md) is
handled by Cuprate's in-house [cuprate-epee-encoding](https://doc.cuprate.org/cuprate_epee_encoding) library.

## Bitcasted `struct`s
> TODO: <https://github.com/monero-project/monero/issues/9422>

## Compressed data
> TODO: <https://github.com/monero-project/monero/issues/9422>