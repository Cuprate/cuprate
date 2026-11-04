# `cuprate-types`
Shared data types within Cuprate.

# Features flags
| Feature flag | Does what |
|--------------|-----------|
| `serde`      | Enables `serde` on types where applicable
| `epee`       | Enables `cuprate-epee-encoding` on types where applicable
| `proptest`   | Enables `proptest::arbitrary::Arbitrary` on some types
| `rpc`        | Enables the `rpc` module, containing types (in)directly used in RPC
