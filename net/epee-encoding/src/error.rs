use core::fmt::{Debug, Formatter};
use core::{num::TryFromIntError, str::Utf8Error};

pub type Result<T> = core::result::Result<T, Error>;

#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum Error {
    #[cfg_attr(feature = "std", error("IO error: {0}"))]
    IO(&'static str),
    #[cfg_attr(feature = "std", error("Format error: {0}"))]
    Format(&'static str),
    #[cfg_attr(feature = "std", error("Value error: {0}"))]
    Value(String),
}

impl Error {
    fn field_name(&self) -> &'static str {
        match self {
            Error::IO(_) => "io",
            Error::Format(_) => "format",
            Error::Value(_) => "value",
        }
    }

    fn field_data(&self) -> &str {
        match self {
            Error::IO(data) => data,
            Error::Format(data) => data,
            Error::Value(data) => data,
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Error")
            .field(self.field_name(), &self.field_data())
            .finish()
    }
}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Self {
        Error::Value("Int is too large".to_string())
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Self {
        Error::Value("Invalid utf8 str".to_string())
    }
}
