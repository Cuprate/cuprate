//! Tests for `cuprate_database`, backed by `heed`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    backend::heed::ConcreteEnv,
    config::{Config, SyncMode},
    database::{DatabaseRo, DatabaseRw},
    env::{Env, EnvInner},
    error::{InitError, RuntimeError},
    resize::ResizeAlgorithm,
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Tests
/// Create a `ConcreteEnv` in a temporarily directory.
/// The directory is automatically removed after the `TempDir` is dropped.
fn tmp_concrete_env() -> (ConcreteEnv, tempfile::TempDir) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = Config::low_power(Some(tempdir.path().into()));
    let env = ConcreteEnv::open(config).unwrap();

    (env, tempdir)
}

/// Simply call [`Env::open`]. If this fails, something is really wrong.
#[test]
fn open() {
    tmp_concrete_env();
}

/// Create database transactions, but don't write any data.
#[test]
fn tx() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();

    env_inner.tx_ro().unwrap().commit().unwrap();
    env_inner.tx_rw().unwrap().commit().unwrap();
    env_inner.tx_rw().unwrap().abort();
}

/// Open (and verify) that all database tables
/// exist already after calling [`Env::open`].
#[test]
fn open_tables() {
    let (env, _tempdir) = tmp_concrete_env();
    let env_inner = env.env_inner();
    let tx_ro = env_inner.tx_ro().unwrap();

    // Open all tables.
    // This should be updated when tables are modified.

    // DatabaseOpenOptions::new(&env)
    //     .name(TestTable::NAME)
    //     .types::<<TestTable as Table>::Key, <TestTable as Table>::Value>()
    //     .create(&mut tx_rw)?;

    // DatabaseOpenOptions::new(&env)
    //     .name(TestTable2::NAME)
    //     .types::<<TestTable2 as Table>::Key, <TestTable2 as Table>::Value>()
    //     .create(&mut tx_rw)?;
}
