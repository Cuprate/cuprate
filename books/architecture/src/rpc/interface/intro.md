# 🟢 The interface
The RPC interface, which includes:

- Endpoint routing (`/json_rpc`, `/get_blocks.bin`, etc)
- Type (de)serialization
- Any miscellaneous handling (denying `restricted` RPC calls)

is handled by the [`cuprate-rpc-interface`](https://doc.cuprate.org/cuprate_rpc_interface) crate.

