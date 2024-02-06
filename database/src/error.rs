//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::backend::DATABASE_BACKEND;

use std::borrow::Cow;
use std::fmt::Debug;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- InitError
/// TODO
///
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
/// TODO: `InitError/RuntimeError` are maybe bad names.
///
/// Database errors that occur _after_ successful initialization.
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

//---------------------------------------------------------------------------------------------------- IMPL

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
