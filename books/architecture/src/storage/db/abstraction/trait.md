# Trait
`cuprate_database` provides a set of `trait`s that abstract over the various database backends.

This allows the function signatures and behavior to stay the same but allows for swapping out databases in an easier fashion.

All common behavior of the backend's are encapsulated here and used instead of using the backend directly.

Examples:
- [`trait Env`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/env.rs)
- [`trait {TxRo, TxRw}`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/transaction.rs)
- [`trait {DatabaseRo, DatabaseRw}`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/database.rs)

For example, instead of calling `heed` or `redb`'s `get()` function directly, `DatabaseRo::get()` is called.

## Usage
With a `ConcreteEnv` and a particular backend selected,
we can now start using it alongside these traits to start
doing database operations in a generic manner.

An example:

```rust
use cuprate_database::{
    ConcreteEnv,
    config::ConfigBuilder,
    Env, EnvInner,
    DatabaseRo, DatabaseRw, TxRo, TxRw,
};

// Initialize the database environment.
let env = ConcreteEnv::open(config)?;

// Open up a transaction + tables for writing.
let env_inner = env.env_inner();
let tx_rw = env_inner.tx_rw()?;
env_inner.create_db::<Table>(&tx_rw)?;

// Write data to the table.
{
	let mut table = env_inner.open_db_rw::<Table>(&tx_rw)?;
	table.put(&0, &1)?;
}

// Commit the transaction.
TxRw::commit(tx_rw)?;
```

As seen above, there is no direct call to `heed` or `redb`.
Their functionality is abstracted behind `ConcreteEnv` and the `trait`s.