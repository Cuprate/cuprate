//! Contains [`TransactionVerificationData`] and the related types.

use std::sync::Mutex as StdMutex;

use monero_serai::transaction::{Timelock, Transaction};

use crate::HardFork;

/// An error when creating a [`TransactionVerificationData`] from a [`Transaction`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TxVerificationDataErr {
    /// The inputs overflow a [`u64`]
    #[error("Tx inputs overflow")]
    InputsOverflow,
    /// The tx outputs more than the inputs sum to.
    #[error("Tx outputs too much")]
    OutputsTooHigh,
    /// The transaction has an invalid version.
    #[error("Tx version invalid")]
    TransactionVersionInvalid,
}

/// An enum representing all valid Monero transaction versions.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TxVersion {
    /// Legacy ring signatures.
    RingSignatures,
    /// Ring-CT
    RingCT,
}

impl TxVersion {
    /// Converts a `raw` version value to a [`TxVersion`].
    ///
    /// This will return `None` on invalid values.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/transactions.html#version>
    ///  &&  <https://monero-book.cuprate.org/consensus_rules/blocks/miner_tx.html#version>
    pub const fn from_raw(version: u8) -> Option<Self> {
        Some(match version {
            1 => Self::RingSignatures,
            2 => Self::RingCT,
            _ => return None,
        })
    }
}

/// Represents if a transaction has been fully validated and under what conditions
/// the transaction is valid in the future.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CachedVerificationState {
    /// The transaction has not been validated.
    NotVerified,
    /// The transaction is valid* if the block represented by this hash is in the blockchain and the [`HardFork`]
    /// is the same.
    ///
    /// *V1 transactions require checks on their ring-length even if this hash is in the blockchain.
    ValidAtHashAndHF {
        /// The block hash that was in the chain when this transaction was validated.
        block_hash: [u8; 32],
        /// The hf this transaction was validated against.
        hf: HardFork,
    },
    /// The transaction is valid* if the block represented by this hash is in the blockchain _and_ this
    /// given time lock is unlocked. The time lock here will represent the youngest used time based lock
    /// (If the transaction uses any time based time locks). This is because time locks are not monotonic
    /// so unlocked outputs could become re-locked.
    ///
    /// *V1 transactions require checks on their ring-length even if this hash is in the blockchain.
    ValidAtHashAndHFWithTimeBasedLock {
        /// The block hash that was in the chain when this transaction was validated.
        block_hash: [u8; 32],
        /// The hf this transaction was validated against.
        hf: HardFork,
        /// The youngest used time based lock.
        time_lock: Timelock,
    },
}

impl CachedVerificationState {
    /// Returns the block hash this is valid for if in state [`CachedVerificationState::ValidAtHashAndHF`] or [`CachedVerificationState::ValidAtHashAndHFWithTimeBasedLock`].
    pub const fn verified_at_block_hash(&self) -> Option<[u8; 32]> {
        match self {
            Self::NotVerified => None,
            Self::ValidAtHashAndHF { block_hash, .. }
            | Self::ValidAtHashAndHFWithTimeBasedLock { block_hash, .. } => Some(*block_hash),
        }
    }
}

/// Data needed to verify a transaction.
#[derive(Debug)]
pub struct TransactionVerificationData {
    /// The transaction we are verifying
    pub tx: Transaction,
    /// The [`TxVersion`] of this tx.
    pub version: TxVersion,
    /// The serialised transaction.
    pub tx_blob: Vec<u8>,
    /// The weight of the transaction.
    pub tx_weight: usize,
    /// The fee this transaction has paid.
    pub fee: u64,
    /// The hash of this transaction.
    pub tx_hash: [u8; 32],
    /// The verification state of this transaction.
    pub cached_verification_state: StdMutex<CachedVerificationState>,
}
