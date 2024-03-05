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
};

//---------------------------------------------------------------------------------------------------- Tests
/// TODO
#[test]
fn test() {
    let concrete_env: ConcreteEnv = todo!();
}
