//! An [`Env`] inside a [`TempDir`].

use tempfile::TempDir;

use cuprate_blockchain::tables::Outputs;
use cuprate_database::{
    config::ConfigBuilder, resize::PAGE_SIZE, ConcreteEnv, DatabaseRw, Env, EnvInner, TxRw,
};

use crate::constants::{KEY, VALUE};

/// A temporary in-memory [`Env`].
///
/// This is a [`ConcreteEnv`] that uses [`TempDir`] as the
/// backing file location - this is an in-memory file on Linux.
pub struct TmpEnv {
    pub env: ConcreteEnv,
    pub tempdir: TempDir,
}

impl Default for TmpEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl TmpEnv {
    /// Create an `Env` in a temporary directory.
    ///
    /// The directory is automatically removed after the [`TempDir`] is dropped.
    #[expect(clippy::missing_panics_doc)]
    pub fn new() -> Self {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().to_path_buf().into();
        let config = ConfigBuilder::new(path).low_power().build();
        let env = ConcreteEnv::open(config).unwrap();

        // Resize to a very large map to prevent resize errors.
        if ConcreteEnv::MANUAL_RESIZE {
            // SAFETY: no write transactions exist yet.
            unsafe {
                env.env_inner()
                    .resize(PAGE_SIZE.get() * 1024 * 1024 * 1024)
                    .unwrap();
            }
        }

        Self { env, tempdir }
    }

    /// Inserts [`KEY`] and [`VALUE`] inside the [`Outputs`] table.
    #[must_use]
    pub fn with_key_value(self) -> Self {
        let env_inner = self.env.env_inner();
        let tx_rw = env_inner.tx_rw().unwrap();
        let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

        table.put(&KEY, &VALUE).unwrap();
        drop(table);
        tx_rw.commit().unwrap();

        drop(env_inner);
        self
    }

    /// Inserts [`VALUE`] inside the [`Outputs`] table 100 times.
    ///
    /// The key is an incrementing [`KEY`], i.e. the keys are
    /// `KEY + {0..99}`, each one has [`VALUE`] as the value.
    #[must_use]
    pub fn with_key_value_100(self) -> Self {
        let env_inner = self.env.env_inner();
        let tx_rw = env_inner.tx_rw().unwrap();
        let mut table = env_inner.open_db_rw::<Outputs>(&tx_rw).unwrap();

        let mut key = KEY;
        for _ in 0..100 {
            table.put(&key, &VALUE).unwrap();
            key.amount += 1;
        }

        drop(table);
        tx_rw.commit().unwrap();

        drop(env_inner);
        self
    }
}
