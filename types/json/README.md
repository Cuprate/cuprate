# `cuprate-json`
JSON representations of common Monero types, to allow for easier JSON (de)serialization.

The main types are:
- [`block::Block`]
- [`tx::Transaction`]

Modules exist within this crate as the JSON representation
of types sometimes differs, thus, the modules hold the types
that match the specific schema, for example [`block::Input`]
is different than [`tx::Input`].

# (De)serialization invariants
These types are only meant for JSON (de)serialization.

Some types implement `epee` although will panic with [`unimplemented`].
See `cuprate_rpc_types` for more information.

# Features flags
| Feature flag | Does what |
|--------------|-----------|
| `serde`      | Enables `serde` on types
| `epee`       | Enables `cuprate-epee-encoding` on types where applicable
