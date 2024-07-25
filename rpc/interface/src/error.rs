//! TODO

//---------------------------------------------------------------------------------------------------- Import

use axum::http::StatusCode;

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {}

impl From<Error> for StatusCode {
    fn from(value: Error) -> Self {
        // TODO
        Self::INTERNAL_SERVER_ERROR
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
