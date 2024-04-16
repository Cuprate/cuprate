//! TODO

// TODO: set-up env based on config

//---------------------------------------------------------------------------------------------------- Import
use cuprate_database::{config::Config, ConcreteEnv, Env};

//---------------------------------------------------------------------------------------------------- Tests
/// Create an `Env` in a temporarily directory.
/// The directory is automatically removed after the `TempDir` is dropped.
///
/// TODO: changing this to `-> impl Env` causes lifetime errors...
#[allow(clippy::missing_panics_doc)]
pub fn tmp_concrete_env() -> (ConcreteEnv, tempfile::TempDir) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = Config::low_power(Some(tempdir.path().into()));
    let env = ConcreteEnv::open(config).unwrap();

    (env, tempdir)
}
