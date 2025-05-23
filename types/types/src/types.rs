//! Various shared data types in Cuprate.

use std::num::NonZero;

use curve25519_dalek::edwards::CompressedEdwardsY;
use monero_serai::{
    block::Block,
    transaction::{Timelock, Transaction},
};

use crate::HardFork;

/// Extended header data of a block.
///
/// This contains various metadata of a block, but not the block blob itself.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtendedBlockHeader {
    /// The block's major version.
    ///
    /// This is the same value as [`monero_serai::block::BlockHeader::hardfork_version`].
    pub version: HardFork,
    /// The block's hard-fork vote.
    ///
    /// This can't be represented with [`HardFork`] as raw-votes can be out of the range of [`HardFork`]s.
    ///
    /// This is the same value as [`monero_serai::block::BlockHeader::hardfork_signal`].
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

/// Verified information of a transaction.
///
/// This represents a valid transaction
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
    /// All the transactions in the block, excluding the [`Block::miner_transaction`].
    pub txs: Vec<VerifiedTransactionInformation>,
    /// The block's hash.
    ///
    /// [`Block::hash`].
    pub block_hash: [u8; 32],
    /// The block's proof-of-work hash.
    // TODO: make this an option.
    pub pow_hash: [u8; 32],
    /// The block's height.
    pub height: usize,
    /// The amount of generated coins (atomic units) in this block.
    pub generated_coins: u64,
    /// The adjusted block size, in bytes.
    pub weight: usize,
    /// The long term block weight, which is the weight factored in with previous block weights.
    pub long_term_weight: usize,
    /// The cumulative difficulty of all blocks up until and including this block.
    pub cumulative_difficulty: u128,
}

/// A unique ID for an alt chain.
///
/// The inner value is meaningless.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ChainId(pub NonZero<u64>);

/// An identifier for a chain.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Chain {
    /// The main chain.
    Main,
    /// An alt chain.
    Alt(ChainId),
}

/// A block on an alternative chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AltBlockInformation {
    /// The block itself.
    pub block: Block,
    /// The serialized byte form of [`Self::block`].
    ///
    /// [`Block::serialize`].
    pub block_blob: Vec<u8>,
    /// All the transactions in the block, excluding the [`Block::miner_transaction`].
    pub txs: Vec<VerifiedTransactionInformation>,
    /// The block's hash.
    ///
    /// [`Block::hash`].
    pub block_hash: [u8; 32],
    /// The block's proof-of-work hash.
    pub pow_hash: [u8; 32],
    /// The block's height.
    pub height: usize,
    /// The adjusted block size, in bytes.
    pub weight: usize,
    /// The long term block weight, which is the weight factored in with previous block weights.
    pub long_term_weight: usize,
    /// The cumulative difficulty of all blocks up until and including this block.
    pub cumulative_difficulty: u128,
    /// The [`ChainId`] of the chain this alt block is on.
    pub chain_id: ChainId,
}

/// An already existing transaction output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputOnChain {
    /// The block height this output belongs to.
    pub height: usize,
    /// The timelock of this output, if any.
    pub time_lock: Timelock,
    /// The public key of this output, if any.
    pub key: CompressedEdwardsY,
    /// The output's commitment.
    pub commitment: CompressedEdwardsY,
    /// The hash of the transaction hash this output belongs to.
    /// This is [`None`] when this field is not requested for in relevant functions.
    pub txid: Option<[u8; 32]>,
}

/// The inner response for a request for txs in a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TxsInBlock {
    pub block: Vec<u8>,
    pub txs: Vec<Vec<u8>>,
}

/// A block template, used in RPC's `get_block_template`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockTemplate {
    pub block: Block,
    pub reserved_offset: u64,
    pub difficulty: u128,
    pub height: u64,
    pub expected_reward: u64,
    pub seed_height: u64,
    pub seed_hash: [u8; 32],
    pub next_seed_hash: [u8; 32],
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TxInBlockchain {
    pub block_height: u64,
    pub block_timestamp: u64,
    pub confirmations: u64,
    pub output_indices: Vec<u64>,
    pub tx_hash: [u8; 32],
    pub tx_blob: Vec<u8>,
    pub pruned_blob: Vec<u8>,
    pub prunable_blob: Vec<u8>,
    pub prunable_hash: [u8; 32],
}

/// TODO
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TxInPool {
    pub tx_blob: Vec<u8>,
    pub tx_hash: [u8; 32],
    pub double_spend_seen: bool,
    pub received_timestamp: u64,
    pub relayed: bool,
}

bitflags::bitflags! {
    pub struct TxRelayChecks: u16 {
        const DOUBLE_SPEND        = 1;
        const FEE_TOO_LOW         = 1 << 1;
        const INVALID_INPUT       = 1 << 2;
        const INVALID_OUTPUT      = 1 << 3;
        const LOW_MIXIN           = 1 << 4;
        const NONZERO_UNLOCK_TIME = 1 << 5;
        const OVERSPEND           = 1 << 6;
        const TOO_BIG             = 1 << 7;
        const TOO_FEW_OUTPUTS     = 1 << 8;
        const TX_EXTRA_TOO_BIG    = 1 << 9;
    }
}

/// Used in RPC's `get_output_distribution`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDistributionInput {
    pub amounts: Vec<u64>,
    pub cumulative: bool,
    pub from_height: u64,

    /// [`None`] means the entire blockchain.
    pub to_height: Option<NonZero<u64>>,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
