//! An [`Env`] inside a [`TempDir`].

use tempfile::TempDir;

use cuprate_blockchain::{
    config::ReaderThreads,
    cuprate_database::{config::ConfigBuilder, resize::PAGE_SIZE, ConcreteEnv, Env},
};

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
        let db_config = ConfigBuilder::new(path).low_power().build();
        let reader_threads = ReaderThreads::One;
        let config = cuprate_blockchain::config::Config {
            db_config,
            reader_threads,
        };
        let env = cuprate_blockchain::open(config).unwrap();

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
}
