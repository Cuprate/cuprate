# JSON field ordering
When serializing JSON, `monerod` has the behavior to order key fields within a scope alphabetically.

For example:
```json
{
  "id": "0",
  "jsonrpc": "2.0",
  "result": {
    "blockhashing_blob": "...",
    "blocktemplate_blob": "...",
    "difficulty": 283305047039,
    "difficulty_top64": 0,
    "expected_reward": 600000000000,
    "height": 3195018,
    "next_seed_hash": "",
    "prev_hash": "9d648e741d85ca0e7acb4501f051b27e9b107d3cd7a3f03aa7f776089117c81a",
    "reserved_offset": 131,
    "seed_hash": "e2aa0b7b55042cd48b02e395d78fa66a29815ccc1584e38db2d1f0e8485cd44f",
    "seed_height": 3194880,
    "status": "OK",
    "untrusted": false,
    "wide_difficulty": "0x41f64bf3ff"
  }
}
```
In the main `{}`, `id` comes before `jsonrpc`, which comes before `result`.

The same alphabetical ordering is applied to the fields within `result`.

Cuprate uses [`serde`](https://docs.rs/serde) for JSON serialization,
which serializes fields based on the _definition_ order, i.e. whatever
order the fields are defined in the code, is the order they will appear
in JSON.

Some `struct` fields within Cuprate's RPC types happen to be alphabetical, but this is not a guarantee.

As these are JSON maps, the ordering of fields should not matter,
although this is something to note as the output will technically differ.

## Example incompatibility
An example of where this leads to incompatibility is if specific
line numbers are depended on to contain specific fields.

For example, this will print the 10th line:
```bash
curl http://127.0.0.1:18081/json_rpc -d '{"jsonrpc":"2.0","id":"0","method":"get_block_template","params":{"wallet_address":"44GBHzv6ZyQdJkjqZje6KLZ3xSyN1hBSFAnLP6EAqJtCRVzMzZmeXTC2AHKDS9aEDTRKmo6a6o9r9j86pYfhCWDkKjbtcns","reserve_size":60}' -H 'Content-Type: application/json' | sed -n 10p
```
It will be `"height": 3195018` in `monerod`'s case, but may not necessarily be for Cuprate.

By all means, this should not be relied upon in the first place, although it is shown as an example.
