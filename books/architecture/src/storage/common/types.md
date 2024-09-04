# Types
## POD types
Since [all types in the database are POD types](../db/serde.md), we must often
provide mappings between outside types and the types actually stored in the database.

A common case is mapping infallible types to and from [`bitflags`](https://docs.rs/bitflag) and/or their raw integer representation.
For example, the [`OutputFlag`](https://doc.cuprate.org/cuprate_blockchain/types/struct.OutputFlags.html) type or `bool` types.

As types like `enum`s, `bool`s and `char`s cannot be casted from an integer infallibly,
`bytemuck::Pod` cannot be implemented on it safely. Thus, we store some infallible version
of it inside the database with a custom type and map them when fetching the data.

## Lean types
Another reason why database crates define their own types is
to cut any unneeded data from the type.

Many of the types used in normal operation (e.g. [`cuprate_types::VerifiedBlockInformation`](https://doc.cuprate.org/cuprate_types/struct.VerifiedBlockInformation.html)) contain lots of extra pre-processed data for convenience.

This would be a waste to store in the database, so in this example, the much leaner
"raw" [`BlockInfo`](https://doc.cuprate.org/cuprate_blockchain/types/struct.BlockInfo.html)
type is stored.
