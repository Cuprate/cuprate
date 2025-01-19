//! Tx-pool [table](crate::tables) types.
//!
//! This module contains all types used by the database tables,
//! and aliases for common  types that use the same underlying
//! primitive type.
//!
//! <!-- FIXME: Add schema here or a link to it when complete -->
use bytemuck::{Pod, Zeroable};
use monero_serai::transaction::Timelock;

use cuprate_types::{CachedVerificationState, HardFork};

/// An inputs key image.
pub type KeyImage = [u8; 32];

/// A transaction hash.
pub type TransactionHash = [u8; 32];

/// A transaction blob hash.
pub type TransactionBlobHash = [u8; 32];

bitflags::bitflags! {
    /// Flags representing the state of the transaction in the pool.
    #[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
    #[repr(transparent)]
    pub struct TxStateFlags: u8 {
        /// A flag for if the transaction is in the stem state.
        const STATE_STEM   = 0b0000_0001;
        /// A flag for if we have seen another tx double spending this tx.
        const DOUBLE_SPENT = 0b0000_0010;
    }
}

/// Information on a tx-pool transaction.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct TransactionInfo {
    /// The transaction's fee.
    pub fee: u64,
    /// The transaction's weight.
    pub weight: usize,
    /// [`TxStateFlags`] of this transaction.
    pub flags: TxStateFlags,
    #[expect(clippy::pub_underscore_fields)]
    /// Explicit padding so that we have no implicit padding bytes in `repr(C)`.
    ///
    /// Allows potential future expansion of this type.
    pub _padding: [u8; 7],
}

/// [`CachedVerificationState`] in a format that can be stored into the database.
///
/// This type impls [`Into`] & [`From`] [`CachedVerificationState`].
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct RawCachedVerificationState {
    /// The raw hash, will be all `0`s if there is no block hash that this is valid for.
    raw_valid_at_hash: [u8; 32],
    /// The raw hard-fork, will be `0` if there is no hf this was validated at.
    raw_hf: u8,
    /// The raw [`u64`] timestamp as little endian bytes ([`u64::to_le_bytes`]).
    ///
    /// This will be `0` if there is no timestamp that needs to be passed for this to
    /// be valid.
    ///
    /// Not a [`u64`] as if it was this type would have an alignment requirement.
    raw_valid_past_timestamp: [u8; 8],
}

impl From<RawCachedVerificationState> for CachedVerificationState {
    fn from(value: RawCachedVerificationState) -> Self {
        // if the hash is all `0`s then there is no hash this is valid at.
        if value.raw_valid_at_hash == [0; 32] {
            return Self::NotVerified;
        }

        let raw_valid_past_timestamp = u64::from_le_bytes(value.raw_valid_past_timestamp);

        // if the timestamp is 0, there is no timestamp that needs to be passed.
        if raw_valid_past_timestamp == 0 {
            return Self::ValidAtHashAndHF {
                block_hash: value.raw_valid_at_hash,
                hf: HardFork::from_version(value.raw_hf)
                    .expect("hard-fork values stored in the DB should always be valid"),
            };
        }

        Self::ValidAtHashAndHFWithTimeBasedLock {
            block_hash: value.raw_valid_at_hash,
            hf: HardFork::from_version(value.raw_hf)
                .expect("hard-fork values stored in the DB should always be valid"),
            time_lock: Timelock::Time(raw_valid_past_timestamp),
        }
    }
}

#[expect(clippy::fallible_impl_from, reason = "only panics in invalid states")]
impl From<CachedVerificationState> for RawCachedVerificationState {
    fn from(value: CachedVerificationState) -> Self {
        match value {
            CachedVerificationState::NotVerified => Self {
                raw_valid_at_hash: [0; 32],
                raw_hf: 0,
                raw_valid_past_timestamp: [0; 8],
            },
            CachedVerificationState::JustSemantic(hf) => todo!(),
            CachedVerificationState::ValidAtHashAndHF { block_hash, hf } => Self {
                raw_valid_at_hash: block_hash,
                raw_hf: hf.as_u8(),
                raw_valid_past_timestamp: [0; 8],
            },
            CachedVerificationState::ValidAtHashAndHFWithTimeBasedLock {
                block_hash,
                hf,
                time_lock,
            } => {
                let Timelock::Time(time) = time_lock else {
                    panic!("ValidAtHashAndHFWithTimeBasedLock timelock was not time-based");
                };

                Self {
                    raw_valid_at_hash: block_hash,
                    raw_hf: hf.as_u8(),
                    raw_valid_past_timestamp: time.to_le_bytes(),
                }
            }
        }
    }
}
