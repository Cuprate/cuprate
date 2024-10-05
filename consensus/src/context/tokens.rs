//! Tokens
//!
//! This module contains tokens which keep track of the validity of certain data.
//! Currently, there is 1 token:
//! - [`ValidityToken`]
//!

use tokio_util::sync::CancellationToken;

/// A token representing if a piece of data is valid.
#[derive(Debug, Clone, Default)]
pub struct ValidityToken {
    token: CancellationToken,
}

impl ValidityToken {
    /// Creates a new [`ValidityToken`]
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Returns `true` if the data is still valid.
    pub fn is_data_valid(&self) -> bool {
        !self.token.is_cancelled()
    }

    /// Sets the data to invalid.
    pub fn set_data_invalid(self) {
        self.token.cancel();
    }
}
