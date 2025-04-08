use alloc::string::{String, ToString};
use core::{
    fmt::{Debug, Formatter},
    num::TryFromIntError,
    str::Utf8Error,
};

pub type Result<T> = core::result::Result<T, Error>;

#[cfg_attr(feature = "std", derive(thiserror::Error))]
#[expect(clippy::error_impl_error, reason = "FIXME: rename this type")]
pub enum Error {
    #[cfg_attr(feature = "std", error("IO error: {0}"))]
    IO(&'static str),
    #[cfg_attr(feature = "std", error("Format error: {0}"))]
    Format(&'static str),
    #[cfg_attr(feature = "std", error("Value error: {0}"))]
    Value(String),
}

impl Error {
    const fn field_name(&self) -> &'static str {
        match self {
            Self::IO(_) => "io",
            Self::Format(_) => "format",
            Self::Value(_) => "value",
        }
    }

    #[expect(
        clippy::missing_const_for_fn,
        reason = "False-postive, `Deref::deref` is not const"
    )]
    fn field_data(&self) -> &str {
        match self {
            Self::IO(data) | Self::Format(data) => data,
            Self::Value(data) => data,
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
        Self::Value("Int is too large".to_string())
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Self {
        Self::Value("Invalid utf8 str".to_string())
    }
}
