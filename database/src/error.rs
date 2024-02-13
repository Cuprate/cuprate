//! Database error types.
//! TODO: `InitError/RuntimeError` are maybe bad names.

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Cow, fmt::Debug};

use crate::constants::DATABASE_BACKEND;

//---------------------------------------------------------------------------------------------------- InitError
/// Database errors that occur during initialization.
///
/// `BackendError` is an error specifically from the
/// database backend being used. TODO: this may not
/// be needed if we can convert all error types into
/// "generic" database errors.
#[derive(thiserror::Error, Debug)]
pub enum InitError<BackendError: Debug> {
    /// TODO
    #[error("database PATH is inaccessible: {0}")]
    Path(std::io::Error),

    /// TODO
    #[error("{DATABASE_BACKEND} error: {0}")]
    Backend(BackendError),

    /// TODO
    ///
    /// An unknown error occurred.
    #[error("unknown error: {0}")]
    Unknown(Cow<'static, str>),
}

//---------------------------------------------------------------------------------------------------- RuntimeError
/// Database errors that occur _after_ successful initialization.
///
/// There are no errors for:
/// 1. Missing tables
/// 2. (De)serialization
///
/// as `cuprate_database` upholds the invariant that:
///
/// 1. All tables exist
/// 2. (De)serialization never fails
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
#[derive(thiserror::Error, Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum RuntimeError<BackendError: Debug> {
    /// The given key already existed in the database.
    ///
    /// The string inside is the output of
    /// [`std::any::type_name`] on the key type.
    #[error("key of type `{0}` already existed")]
    KeyExists(&'static str),

    /// The given key did not exist in the database.
    ///
    /// The string inside is the output of
    /// [`std::any::type_name`] on the key type.
    #[error("key/value pair was not found: {0}")]
    KeyNotFound(&'static str),

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

    /// A [`std::io::Error`].
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The expected database version was not the version found.
    #[error("database version mismatch: expected {expected}, found {found}")]
    VersionMismatch {
        expected: &'static str,
        found: &'static str,
    },

    /// The database has reached maximum parallel readers.
    ///
    /// TODO: this can be used for retry logic in reader threads,
    /// although, does this error ever actually occur in practice?
    #[error("database maximum parallel readers reached")]
    MaxReaders,

    /// An unknown backend-specific error occured.
    #[error("{DATABASE_BACKEND} error: {0}")]
    Backend(BackendError),

    // TODO: this could be removed once we have all errors figured out.
    /// An unknown error occurred.
    #[error("unknown error: {0}")]
    Unknown(Cow<'static, str>),
}
