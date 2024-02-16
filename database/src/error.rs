//! Database error types.
//! TODO: `InitError/RuntimeError` are maybe bad names.

//---------------------------------------------------------------------------------------------------- Import
use std::fmt::Debug;

#[allow(unused_imports)] // docs
use crate::env::Env;

//---------------------------------------------------------------------------------------------------- Types
/// Alias for a thread-safe boxed error.
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

//---------------------------------------------------------------------------------------------------- InitError
/// Errors that occur during ([`Env::open`]).
#[derive(thiserror::Error, Debug)]
pub enum InitError {
    /// The given `Path/File` existed and was accessible,
    /// but was not a valid database file.
    #[error("database file exists but is not valid")]
    Invalid,

    /// The given `Path/File` existed, was a valid
    /// database, but the version is incorrect.
    #[error("database file is valid, but version is incorrect")]
    InvalidVersion,

    /// I/O error.
    #[error("database I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The given `Path/File` existed,
    /// was a valid database, but it is corrupt.
    #[error("database file is corrupt")]
    Corrupt,

    /// The database is currently in the process
    /// of shutting down and cannot respond.
    ///
    /// TODO: This might happen if we try to open
    /// while we are shutting down, `unreachable!()`?
    #[error("database is shutting down")]
    ShuttingDown,

    /// A fatal error occurred.
    ///
    /// This is for errors that _should_ be unreachable
    /// but we'd still like to panic gracefully.
    ///
    /// # Invariant
    /// If this error is received, all(?) of Cuprate should shutdown.
    #[error("fatal error: {0}")]
    Fatal(BoxError),

    /// An unknown error occurred.
    ///
    /// This is a catch-all for all non-fatal errors.
    #[error("unknown error: {0}")]
    Unknown(BoxError),
}

//---------------------------------------------------------------------------------------------------- RuntimeError
/// Errors that occur _after_ successful ([`Env::open`]).
///
/// There are no errors for:
/// 1. Missing tables
/// 2. (De)serialization
///
/// as `cuprate_database` upholds the invariant that:
///
/// 1. All tables exist
/// 2. (De)serialization never fails
#[derive(thiserror::Error, Debug)]
pub enum RuntimeError {
    /// The given key already existed in the database.
    #[error("key already existed")]
    KeyExists,

    /// The given key did not exist in the database.
    #[error("key/value pair was not found")]
    KeyNotFound,

    /// A [`std::io::Error`].
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The database is currently in the process
    /// of shutting down and cannot respond.
    #[error("database is shutting down")]
    ShuttingDown,
}
