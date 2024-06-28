//! Utilities for `cuprate_database` testing.
//!
//! These types/fn's are only:
//! - enabled on #[cfg(test)]
//! - only used internally

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::Cow;

use crate::{config::ConfigBuilder, table::Table, ConcreteEnv, Env};

//---------------------------------------------------------------------------------------------------- struct
/// A test table.
pub(crate) struct TestTable;

impl Table for TestTable {
    const NAME: &'static str = "test_table";
    type Key = u32;
    type Value = u64;
}

//---------------------------------------------------------------------------------------------------- fn
/// Create an `Env` in a temporarily directory.
/// The directory is automatically removed after the `TempDir` is dropped.
///
/// FIXME: changing this to `-> impl Env` causes lifetime errors...
pub(crate) fn tmp_concrete_env() -> (ConcreteEnv, tempfile::TempDir) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = ConfigBuilder::new(Cow::Owned(tempdir.path().into()))
        .low_power()
        .build();
    let env = ConcreteEnv::open(config).unwrap();

    (env, tempdir)
}
