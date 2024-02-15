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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
#[derive(thiserror::Error, Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum RuntimeError {
    // TODO: replace string with actual error type.
    ///
    /// An error occurred when attempting to
    /// serialize the key data into bytes.
    #[error("serialize error: {0}")]
    Serialize(String),

    // TODO: replace string with actual error type.
    ///
    /// An error occurred when attempting to
    /// deserialize the response value from
    /// the database.
    #[error("deserialize error: {0}")]
    Deserialize(String),

    /// TODO
    ///
    /// An unknown error occurred.
    #[error("unknown error: {0}")]
    Unknown(Cow<'static, str>),
}
