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
Different database backends are enabled by the feature flags:

- `heed` (LMDB)
- `redb`

The default is `heed`.

`tracing` is always enabled and cannot be disabled via feature-flag.
<!-- FIXME: tracing should be behind a feature flag -->

# Invariants when not using `service`

See [`cuprate_blockchain`](https://doc.cuprate.org/cuprate_blockchain), the invariants are the same.

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
