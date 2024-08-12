Cuprate's tx-pool database.

This documentation is mostly for practical usage of `cuprate_txpool`.

For a high-level overview, see the database section in
[Cuprate's architecture book](https://architecture.cuprate.org).

If you're looking for a database crate, consider using the lower-level
[`cuprate-database`](https://doc.cuprate.org/cuprate_database)
crate that this crate is built on-top of.

# Purpose

This crate does 3 things:

1. Uses [`cuprate_database`] as a base database layer
1. Implements various transaction pool related [operations](ops), [tables], and [types]
1. Exposes a [`tower::Service`] backed by a thread-pool

Each layer builds on-top of the previous.

As a user of `cuprate_txpool`, consider using the higher-level [`service`] module,
or at the very least the [`ops`] module instead of interacting with the `cuprate_database` traits directly.

# `cuprate_database`

Consider reading `cuprate_database`'s crate documentation before this crate, as it is the first layer.

If/when this crate needs is used, be sure to use the version that this crate re-exports, e.g.:

```rust
use cuprate_txpool::{
    cuprate_database::RuntimeError,
};
```

This ensures the types/traits used from `cuprate_database` are the same ones used by `cuprate_txpool` internally.

# Feature flags

The `service` module requires the `service` feature to be enabled.
See the module for more documentation.

Different database backends are enabled by the feature flags:

- `heed` (LMDB)
- `redb`

The default is `heed`.

`tracing` is always enabled and cannot be disabled via feature-flag.
<!-- FIXME: tracing should be behind a feature flag -->

# Invariants when not using `service`

`cuprate_txpool` can be used without the `service` feature enabled but
there are some things that must be kept in mind when doing so.

Failing to uphold these invariants may cause panics.

1. `LMDB` requires the user to resize the memory map resizing (see [`cuprate_database::RuntimeError::ResizeNeeded`]
1. `LMDB` has a maximum reader transaction count,
   currently, [it is set to `126`](https://github.com/LMDB/lmdb/blob/b8e54b4c31378932b69f1298972de54a565185b1/libraries/liblmdb/mdb.c#L794-L799)
1. `LMDB`
   has [maximum key/value byte size](http://www.lmdb.tech/doc/group__internal.html#gac929399f5d93cef85f874b9e9b1d09e0)
   which must not be exceeded

# Examples

The below is an example of using `cuprate_txpool`'s
lowest API, i.e. using a mix of this crate and `cuprate_database`'s traits directly -
**this is NOT recommended.**

For examples of the higher-level APIs, see:

- [`ops`]
- [`service`]

```rust
use cuprate_txpool::{
    cuprate_database::{
        ConcreteEnv,
        Env, EnvInner,
        DatabaseRo, DatabaseRw, TxRo, TxRw,
    },
    config::ConfigBuilder,
    tables::{Tables, TablesMut, OpenTables},
};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a configuration for the database environment.
    let tmp_dir = tempfile::tempdir()?;
    let db_dir = tmp_dir.path().to_owned();
    let config = ConfigBuilder::new()
        .db_directory(db_dir.into())
        .build();

    // Initialize the database environment.
    let env = cuprate_txpool::open(config)?;

    // Open up a transaction + tables for writing.
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;
    let mut tables = env_inner.open_tables_mut(&tx_rw)?;

    // ⚠️ Write data to the tables directly.
    // (not recommended, use `ops` or `service`).
    const KEY_IMAGE: [u8; 32] = [88; 32];
    const TX_HASH: [u8; 32] = [88; 32];
    tables.spent_key_images_mut().put(&KEY_IMAGE, &TX_HASH)?;

    // Commit the data written.
    drop(tables);
    TxRw::commit(tx_rw)?;

    // Read the data, assert it is correct.
    let tx_ro = env_inner.tx_ro()?;
    let tables = env_inner.open_tables(&tx_ro)?;
    let (key_image, tx_hash) = tables.spent_key_images().first()?;
    assert_eq!(key_image, KEY_IMAGE);
    assert_eq!(tx_hash, TX_HASH);
    # Ok(())
}
```
