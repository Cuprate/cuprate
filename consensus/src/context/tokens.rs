use tokio_util::sync::CancellationToken;

/// A token representing if a piece of data is valid.
#[derive(Debug, Clone, Default)]
pub struct ValidityToken {
    token: CancellationToken,
}

impl ValidityToken {
    pub fn new() -> ValidityToken {
        ValidityToken {
            token: CancellationToken::new(),
        }
    }

    pub fn is_data_valid(&self) -> bool {
        !self.token.is_cancelled()
    }

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
    pub fn new() -> ReOrgToken {
        ReOrgToken {
            token: CancellationToken::new(),
        }
    }

    pub fn reorg_happened(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn set_reorg_happened(self) {
        self.token.cancel()
    }
}
