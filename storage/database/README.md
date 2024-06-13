# `database`

Cuprate's database abstraction.

This documentation is mostly for practical usage of `database`.

For a high-level overview, see the database section in
[Cuprate's architecture book](https://architecture.cuprate.org).

# Purpose
This crate abstracts various database backends with traits.

If you need blockchain specific capabilities, consider using the higher-level
`cuprate-blockchain` crate which builds upon this one.

# Terminology
To be more clear on some terms used in this crate:

| Term             | Meaning                              |
|------------------|--------------------------------------|
| `Env`            | The 1 database environment, the "whole" thing
| `DatabaseR{o,w}` | A _actively open_ readable/writable `key/value` store
| `Table`          | Solely the metadata of a `Database` (the `key` and `value` types, and the name)
| `TxR{o,w}`       | A read/write transaction
| `Storable`       | A data that type can be stored in the database

The dataflow is `Env` -> `Tx` -> `Database`

Which reads as:
1. You have a database `Environment`
1. You open up a `Transaction`
1. You open a particular `Table` from that `Environment`, getting a `Database`
1. You can now read/write data from/to that `Database`

# Concrete types
You should _not_ rely on the concrete type of any abstracted backend.

For example, when using the `heed` backend, [`Env`]'s associated [`TxRw`] type
is `RefCell<heed::RwTxn<'_>>`. In order to ensure compatibility with other backends
and to not create backend-specific code, you should _not_ refer to that concrete type.

Use generics and trait notation in these situations:
- `impl<T: TxRw> Trait for Object`
- `fn() -> impl TxRw`

# `ConcreteEnv`
This crate exposes [`ConcreteEnv`], which is a non-generic/non-dynamic,
concrete object representing a database [`Env`]ironment.

The actual backend for this type is determined via feature flags.

This object existing means `E: Env` doesn't need to be spread all through the codebase,
however, it also means some small invariants should be kept in mind.

As `ConcreteEnv` is just a re-exposed type which has varying inner types,
it means some properties will change depending on the backend used.

For example:
- [`std::mem::size_of::<ConcreteEnv>`]
- [`std::mem::align_of::<ConcreteEnv>`]

Things like these functions are affected by the backend and inner data,
and should not be relied upon. This extends to any `struct/enum` that contains `ConcreteEnv`.

`ConcreteEnv` invariants you can rely on:
- It implements [`Env`]
- Upon [`Drop::drop`], all database data will sync to disk

Note that `ConcreteEnv` itself is not a clonable type,
it should be wrapped in [`std::sync::Arc`].

<!-- SOMEDAY: replace `ConcreteEnv` with `fn Env::open() -> impl Env`/
and use `<E: Env>` everywhere it is stored instead. This would allow
generic-backed dynamic runtime selection of the database backend, i.e.
the user can select which database backend they use. -->

# Feature flags
Different database backends are enabled by the feature flags:
- `heed` (LMDB)
- `redb`

The default is `heed`.

`tracing` is always enabled and cannot be disabled via feature-flag.
<!-- FIXME: tracing should be behind a feature flag -->

# Examples
The below is an example of using `cuprate-database`.

```rust
use cuprate_database::{
    ConcreteEnv,
    config::ConfigBuilder,
    Env, EnvInner,
    DatabaseRo, DatabaseRw, TxRo, TxRw,
};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Create a configuration for the database environment.
let db_dir = tempfile::tempdir()?;
let config = ConfigBuilder::new(db_dir.path().to_path_buf()).build();

// Initialize the database environment.
let env = ConcreteEnv::open(config)?;

// Define metadata for a table.
struct Table;
impl cuprate_database::Table for Table {
    // The name of the table is "table".
    const NAME: &'static str = "table";
    // The key type is a `u8`.
    type Key = u8;
    // The key type is a `u64`.
    type Value = u64;
}

// Open up a transaction + tables for writing.
let env_inner = env.env_inner();
let tx_rw = env_inner.tx_rw()?;
// We must create the table first or the next line will error.
env_inner.create_db::<Table>(&tx_rw)?;
let mut table = env_inner.open_db_rw::<Table>(&tx_rw)?;

// Write data to the table.
table.put(&0, &1)?;

// Commit the data written.
drop(table);
TxRw::commit(tx_rw)?;

// Read the data, assert it is correct.
let tx_ro = env_inner.tx_ro()?;
let table = env_inner.open_db_ro::<Table>(&tx_ro)?;
assert_eq!(table.first()?, (0, 1));
# Ok(()) }
```