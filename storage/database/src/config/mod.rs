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
//! use database::{
//!     ConcreteEnv, Env,
//!     config::{ConfigBuilder, ReaderThreads, SyncMode}
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let db_dir = tempfile::tempdir()?;
//!
//! let config = ConfigBuilder::new()
//!      // Use a custom database directory.
//!     .db_directory(db_dir.path().to_path_buf())
//!     // Use as many reader threads as possible (when using `service`).
//!     .reader_threads(ReaderThreads::OnePerThread)
//!     // Use the fastest sync mode.
//!     .sync_mode(SyncMode::Fast)
//!     // Build into `Config`
//!     .build();
//!
//! // Open the database `service` using this configuration.
//! let env = ConcreteEnv::open(config.clone())?;
//! // It's using the config we provided.
//! assert_eq!(env.config(), &config);
//! # Ok(()) }
//! ```

mod config;
pub use config::{Config, ConfigBuilder};

mod reader_threads;
pub use reader_threads::ReaderThreads;

mod sync_mode;
pub use sync_mode::SyncMode;
