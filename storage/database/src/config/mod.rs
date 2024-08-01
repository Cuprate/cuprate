//! Database [`Env`](crate::Env) configuration.
//!
//! This module contains the main [`Config`]uration struct
//! for the database [`Env`](crate::Env)ironment, and types
//! related to configuration settings.
//!
//! The main constructor is the [`ConfigBuilder`].
//!
//! These configurations are processed at runtime, meaning
//! the `Env` can/will dynamically adjust its behavior
//! based on these values.
//!
//! # Example
//! ```rust
//! use cuprate_database::{
//!     Env,
//!     config::{ConfigBuilder, SyncMode}
//! };
//!
//! #[cfg(feature = "heed")]
//! use cuprate_database::HeedEnv as ConcreteEnv;
//! #[cfg(all(feature = "redb", not(feature = "heed")))]
//! use cuprate_database::RedbEnv as ConcreteEnv;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let db_dir = tempfile::tempdir()?;
//!
//! let config = ConfigBuilder::new(db_dir.path().to_path_buf().into())
//!     // Use the fastest sync mode.
//!     .sync_mode(SyncMode::Fast)
//!     // Build into `Config`
//!     .build();
//!
//! // Open the database using this configuration.
//! let env = ConcreteEnv::open(config.clone())?;
//! // It's using the config we provided.
//! assert_eq!(env.config(), &config);
//! # Ok(()) }
//! ```

mod config;
pub use config::{Config, ConfigBuilder, READER_THREADS_DEFAULT};

mod sync_mode;
pub use sync_mode::SyncMode;
