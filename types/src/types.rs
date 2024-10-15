//! Various shared data types in Cuprate.

use std::num::NonZero;

use curve25519_dalek::edwards::EdwardsPoint;
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
    pub key: Option<EdwardsPoint>,
    /// The output's commitment.
    pub commitment: EdwardsPoint,
}

/// Input required to generate an output histogram.
///
/// Used in RPC's `get_output_histogram`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutputHistogramInput {
    pub amounts: Vec<u64>,
    pub min_count: u64,
    pub max_count: u64,
    pub unlocked: bool,
    pub recent_cutoff: u64,
}

/// A single entry in an output histogram.
///
/// Used in RPC's `get_output_histogram`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutputHistogramEntry {
    pub amount: u64,
    pub total_instances: u64,
    pub unlocked_instances: u64,
    pub recent_instances: u64,
}

/// Data of summed coinbase transactions.
///
/// Used in RPC's `get_coinbase_tx_sum`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoinbaseTxSum {
    pub emission_amount: u128,
    pub fee_amount: u128,
}

/// Data to create a custom block template.
///
/// Used in RPC's `get_miner_data`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MinerData {
    pub major_version: u8,
    pub height: u64,
    pub prev_id: [u8; 32],
    pub seed_hash: [u8; 32],
    pub difficulty: u128,
    pub median_weight: u64,
    pub already_generated_coins: u64,
    pub tx_backlog: Vec<MinerDataTxBacklogEntry>,
}

/// A transaction in the txpool.
///
/// Used in [`MinerData::tx_backlog`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MinerDataTxBacklogEntry {
    pub id: [u8; 32],
    pub weight: u64,
    pub fee: u64,
}

/// Information on a [`HardFork`].
///
/// Used in RPC's `hard_fork_info`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HardForkInfo {
    pub earliest_height: u64,
    pub enabled: bool,
    pub state: u32,
    pub threshold: u32,
    pub version: u8,
    pub votes: u32,
    pub voting: u8,
    pub window: u32,
}

/// Estimated fee data.
///
/// Used in RPC's `get_fee_estimate`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeeEstimate {
    pub fee: u64,
    pub fees: Vec<u64>,
    pub quantization_mask: u64,
}

/// Information on a (maybe alternate) chain.
///
/// Used in RPC's `get_alternate_chains`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainInfo {
    pub block_hash: [u8; 32],
    pub block_hashes: Vec<[u8; 32]>,
    pub difficulty: u128,
    pub height: u64,
    pub length: u64,
    pub main_chain_parent_block: [u8; 32],
}

/// Used in RPC's `add_aux_pow`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuxPow {
    pub id: [u8; 32],
    pub hash: [u8; 32],
}

/// Used in RPC's `add_aux_pow`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddAuxPow {
    pub blocktemplate_blob: Vec<u8>,
    pub blockhashing_blob: Vec<u8>,
    pub merkle_root: [u8; 32],
    pub merkle_tree_depth: u64,
    pub aux_pow: Vec<AuxPow>,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
