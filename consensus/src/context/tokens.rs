//! Tokens
//!
//! This module contains tokens which keep track of the validity of certain data.
//! Currently there are 2 tokens:
//! - [`ValidityToken`]
//! - [`ReOrgToken`]
//!

use tokio_util::sync::CancellationToken;

/// A token representing if a piece of data is valid.
#[derive(Debug, Clone, Default)]
pub struct ValidityToken {
    token: CancellationToken,
}

impl ValidityToken {
    /// Creates a new [`ValidityToken`]
    pub fn new() -> ValidityToken {
        ValidityToken {
            token: CancellationToken::new(),
        }
    }

    /// Returns if the data is still valid.
    pub fn is_data_valid(&self) -> bool {
        !self.token.is_cancelled()
    }

    /// Sets the data to invalid.
    pub fn set_data_invalid(self) {
        self.token.cancel()
    }
}

/// A token representing if a re-org has happened since it's creation.
#[derive(Debug, Clone, Default)]
pub struct ReOrgToken {
    token: CancellationToken,
}

impl ReOrgToken {
    /// Creates a new [`ReOrgToken`].
    pub fn new() -> ReOrgToken {
        ReOrgToken {
            token: CancellationToken::new(),
        }
    }

    /// Returns if a reorg has happened.
    pub fn reorg_happened(&self) -> bool {
        self.token.is_cancelled()
    }

    /// This function tells all reorg tokens related to it that a reorg has happened.
    pub fn set_reorg_happened(self) {
        self.token.cancel()
    }
}
