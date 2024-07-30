//! Various shared data types in Cuprate.

//---------------------------------------------------------------------------------------------------- Import
use std::sync::Mutex as StdMutex;

use curve25519_dalek::edwards::EdwardsPoint;
use monero_serai::{
    block::Block,
    ringct::RctType,
    transaction::{Timelock, Transaction},
};

use crate::hard_fork::HardFork;

//---------------------------------------------------------------------------------------------------- ExtendedBlockHeader
/// Extended header data of a block.
///
/// This contains various metadata of a block, but not the block blob itself.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtendedBlockHeader {
    /// The block's major version, also the hard-fork of the block.
    pub version: HardFork,
    /// The block's hard-fork vote.
    ///
    /// This can't be represented using [`HardFork`] as blocks can vote for future HFs unknown to our node.
    ///
    /// This is the same value as [`monero_serai::block::BlockHeader::minor_version`].
    pub vote: u8,
    /// The UNIX time at which the block was mined.
    pub timestamp: u64,
    /// The total amount of coins mined in all blocks so far, including this block's.
    pub cumulative_difficulty: u128,
    /// The adjusted block size, in bytes.
    pub block_weight: usize,
    /// The long term block weight, based on the median weight of the preceding `100_000` blocks.
    pub long_term_weight: usize,
}

//---------------------------------------------------------------------------------------------------- VerifiedTransactionInformation
/// Verified information of a transaction.
///
/// This represents a transaction in a valid block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedTransactionInformation {
    /// The transaction itself.
    pub tx: Transaction,
    /// The serialized byte form of [`Self::tx`].
    ///
    /// [`Transaction::serialize`].
    pub tx_blob: Vec<u8>,
    /// The transaction's weight.
    ///
    /// [`Transaction::weight`].
    pub tx_weight: usize,
    /// The transaction's total fees.
    pub fee: u64,
    /// The transaction's hash.
    ///
    /// [`Transaction::hash`].
    pub tx_hash: [u8; 32],
}

//---------------------------------------------------------------------------------------------------- VerifiedBlockInformation
/// Verified information of a block.
///
/// This represents a block that has already been verified to be correct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedBlockInformation {
    /// The block itself.
    pub block: Block,
    /// The serialized byte form of [`Self::block`].
    ///
    /// [`Block::serialize`].
    pub block_blob: Vec<u8>,
    /// All the transactions in the block, excluding the [`Block::miner_tx`].
    pub txs: Vec<VerifiedTransactionInformation>,
    /// The block's hash.
    ///
    /// [`Block::hash`].
    pub block_hash: [u8; 32],
    /// The block's proof-of-work hash.
    pub pow_hash: [u8; 32],
    /// The block's height.
    pub height: u64,
    /// The amount of generated coins (atomic units) in this block.
    pub generated_coins: u64,
    /// The adjusted block size, in bytes.
    pub weight: usize,
    /// The long term block weight, which is the weight factored in with previous block weights.
    pub long_term_weight: usize,
    /// The cumulative difficulty of all blocks up until and including this block.
    pub cumulative_difficulty: u128,
}

//---------------------------------------------------------------------------------------------------- OutputOnChain
/// An already existing transaction output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputOnChain {
    /// The block height this output belongs to.
    pub height: u64,
    /// The timelock of this output, if any.
    pub time_lock: Timelock,
    /// The public key of this output, if any.
    pub key: Option<EdwardsPoint>,
    /// The output's commitment.
    pub commitment: EdwardsPoint,
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

/// Data needed to verify a transaction.
#[derive(Debug)]
pub struct TransactionVerificationData {
    /// The transaction we are verifying
    pub tx: Transaction,
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

impl TransactionVerificationData {
    /// Creates a new [`TransactionVerificationData`] from the given [`Transaction`].
    pub fn new(tx: Transaction) -> TransactionVerificationData {
        let tx_hash = tx.hash();
        let tx_blob = tx.serialize();

        // the tx weight is only different from the blobs length for bp(+) txs.
        let tx_weight = match tx.rct_signatures.rct_type() {
            RctType::Bulletproofs
            | RctType::BulletproofsCompactAmount
            | RctType::Clsag
            | RctType::BulletproofsPlus => tx.weight(),
            _ => tx_blob.len(),
        };

        TransactionVerificationData {
            tx_hash,
            tx_blob,
            tx_weight,
            fee: tx.rct_signatures.base.fee,
            cached_verification_state: StdMutex::new(CachedVerificationState::NotVerified),
            tx,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
