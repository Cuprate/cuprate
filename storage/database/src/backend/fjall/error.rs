use fjall::Error;
use crate::{InitError, RuntimeError};

impl From<fjall::Error> for InitError {
    fn from(e: fjall::Error) -> Self {
        InitError::Unknown(e.into())
    }
}

impl From<fjall::Error> for RuntimeError {
    fn from(e: fjall::Error) -> Self {
        Self::TableNotFound
    }
}