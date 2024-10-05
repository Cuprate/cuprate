# `cuprate-types`
Shared data types within Cuprate.

This crate is a kitchen-sink for data types that are shared across Cuprate.

# Features flags
| Feature flag | Does what |
|--------------|-----------|
| `blockchain` | Enables the `blockchain` module, containing the blockchain database request/response types
| `serde`      | Enables `serde` on types where applicable
| `epee`       | Enables `cuprate-epee-encoding` on types where applicable
| `proptest`   | Enables `proptest::arbitrary::Arbitrary` on some types
| `json`       | Enables the `json` module, containing JSON representations of common Monero types
| `hex`        | Enables the `hex` module, containing the `HexBytes` type