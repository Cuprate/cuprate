//! Fallback route functions.

//---------------------------------------------------------------------------------------------------- Import
use axum::http::StatusCode;

//---------------------------------------------------------------------------------------------------- Routes
/// Fallback route function.
///
/// This is used as the fallback endpoint in [`crate::RouterBuilder`].
pub(crate) async fn fallback() -> StatusCode {
    tracing::info!("Routing fallback route");
    StatusCode::NOT_FOUND
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
