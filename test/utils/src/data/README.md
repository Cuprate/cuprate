# Data
This module contains:
- Raw binary, hex, or JSON data for testing purposes
- Functions to access that data, either raw or typed

- `.bin` is a data blob, directly deserializable into types, e.g. `monero_serai::block::Block::read::<&[u8]>(&mut blob)`
- `.hex` is just a hex string of the blob
- `.json` is just the data in regular JSON form (as it would be from a JSON-RPC response)

# Actual data
| Directory | File naming scheme           | Example |
|-----------|------------------------------|---------|
| `block/`  | `$block_hash.{bin,hex,json}` | `bbd604d2ba11ba27935e006ed39c9bfdd99b76bf4a50654bc1e1e61217962698.bin`
| `tx/`     | `$tx_hash.{bin,hex,json}`    | `84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66.bin`
