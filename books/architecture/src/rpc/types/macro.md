# The type generator macro
Request and response types make up the majority of [`cuprate-rpc-types`](https://doc.cuprate.org/cuprate_rpc_types).

- Request types are the input expected _from_ users
- Response types are the data Cuprate will output _to_ users

Regardless of being meant for JSON-RPC, binary, or a standalone JSON endpoint,
all request/response types are defined using the ["type generator macro"](https://github.com/Cuprate/cuprate/blob/bd375eae40acfad7c8d0205bb10afd0b78e424d2/rpc/types/src/macros.rs#L46). This macro is important because it defines _all_ request/response types.

This macro:
- Defines a matching pair of request & response types
- Implements many `derive` traits, e.g. `Clone` on those types
- Implements both `serde` and `epee` on those types
- Automates documentation, tests, etc.

See [here](https://github.com/Cuprate/cuprate/blob/bd375eae40acfad7c8d0205bb10afd0b78e424d2/rpc/types/src/macros.rs#L46) for example usage of this macro.