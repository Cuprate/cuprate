//! Tests for `cuprate_database`, backed by `heed`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    backend::heed::ConcreteEnv,
    config::{Config, SyncMode},
    database::{DatabaseRo, DatabaseRw},
    env::Env,
    error::{InitError, RuntimeError},
    resize::ResizeAlgorithm,
    table::Table,
    transaction::TxCreator,
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

/// Create database transactions, but write any data.
#[test]
fn tx() {
    let (env, _tempdir) = tmp_concrete_env();
    let tx_creator = env.tx_creator();

    tx_creator.tx_ro().unwrap().commit().unwrap();
    tx_creator.tx_rw().unwrap().commit().unwrap();
    tx_creator.tx_rw().unwrap().abort();
}
