//! TODO
#![allow(clippy::unused_async)] // TODO: remove after impl

//---------------------------------------------------------------------------------------------------- Import
use axum::http::StatusCode;

//---------------------------------------------------------------------------------------------------- Routes
/// TODO
pub(crate) async fn unknown() -> StatusCode {
    StatusCode::NOT_FOUND
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
