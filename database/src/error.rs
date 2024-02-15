//! Database error types.
//! TODO: `InitError/RuntimeError` are maybe bad names.

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, fmt::Debug};

#[allow(unused_imports)] // docs
use crate::env::Env;

//---------------------------------------------------------------------------------------------------- InitError
/// Errors that occur during ([`Env::open`]).
#[derive(thiserror::Error, Debug)]
pub enum InitError {
    /// I/O error.
    #[error("database I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The given `Path/File` existed and was accessible,
    /// but was not a valid database file.
    #[error("database file exists but is not valid")]
    Invalid,

    /// The given `Path/File` existed, was a valid
    /// database, but the version is incorrect.
    #[error("database file is valid, but version is incorrect")]
    InvalidVersion,

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

    // TODO: this could be removed once we have all errors figured out.
    /// An unknown error occurred.
    #[error("unknown error: {0}")]
    Unknown(Cow<'static, str>),
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

    /// The database environment has reached
    /// maximum memory map size, it must be
    /// increased.
    //
    // TODO: `sanakirja` automatically resizes, `heed` does not.
    // I guess this should be `unreachable!()` for `sanakirja`?
    #[error("not enough space in database environment memory map")]
    MapFull,

    /// A database page does not have enough
    /// space for more key/values.
    #[error("not enough space in database page")]
    PageFull,

    /// Unknown error, the transaction should abort.
    ///
    /// TODO: this is for: <https://docs.rs/heed/latest/heed/enum.MdbError.html#variant.BadTxn>
    /// Can we even recover here? Should we panic?
    #[error("transaction error, must abort")]
    TxMustAbort,

    /// A [`std::io::Error`].
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The database is currently in the process
    /// of shutting down and cannot respond.
    #[error("database is shutting down")]
    ShuttingDown,

    /// The expected database version was not the version found.
    #[error("database version mismatch")]
    InvalidVersion,

    /// The database has reached maximum parallel readers.
    ///
    /// TODO: this can be used for retry logic in reader threads,
    /// although, does this error ever actually occur in practice?
    #[error("database maximum parallel readers reached")]
    ReadersFull,

    /// The database is corrupt.
    ///
    /// TODO: who knows what this means - is it safe to say
    /// the database is unusable if this error surfaces?
    /// Should we tell the user they have to resync from scratch?
    /// <https://docs.rs/heed/latest/heed/enum.MdbError.html#variant.Corrupted>
    /// <https://docs.rs/sanakirja/latest/sanakirja/enum.Error.html#variant.Corrupt>
    #[error("database is corrupt")]
    Corrupt,

    // TODO: this could be removed once we have all errors figured out.
    /// An unknown error occurred.
    #[error("unknown error: {0}")]
    Unknown(Cow<'static, str>),
}
