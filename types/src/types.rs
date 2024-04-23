//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::sync::Arc;

use curve25519_dalek::edwards::EdwardsPoint;
use monero_serai::{
    block::Block,
    transaction::{Timelock, Transaction},
};

#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

//---------------------------------------------------------------------------------------------------- ExtendedBlockHeader
/// TODO
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub struct ExtendedBlockHeader {
    /// TODO
    /// This is a `cuprate_consensus::HardFork`.
    pub version: u8,
    /// TODO
    /// This is a `cuprate_consensus::HardFork`.
    pub vote: u8,
    /// TODO
    pub timestamp: u64,
    /// TODO
    pub cumulative_difficulty: u128,
    /// TODO
    pub block_weight: usize,
    /// TODO
    pub long_term_weight: usize,
}

//---------------------------------------------------------------------------------------------------- TransactionVerificationData
/// TODO
#[derive(Clone, Debug, PartialEq, Eq)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))] // FIXME: monero_serai
// #[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub struct TransactionVerificationData {
    /// TODO
    pub tx: Transaction,
    /// TODO
    pub tx_blob: Vec<u8>,
    /// TODO
    pub tx_weight: usize,
    /// TODO
    pub fee: u64,
    /// TODO
    pub tx_hash: [u8; 32],
}

//---------------------------------------------------------------------------------------------------- VerifiedBlockInformation
/// TODO
#[derive(Clone, Debug, PartialEq, Eq)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))] // FIXME: monero_serai
// #[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub struct VerifiedBlockInformation {
    /// TODO
    pub block: Block,
    /// TODO
    pub txs: Vec<Arc<TransactionVerificationData>>,
    /// TODO
    pub block_hash: [u8; 32],
    /// TODO
    pub pow_hash: [u8; 32],
    /// TODO
    pub height: u64,
    /// TODO
    pub generated_coins: u64,
    /// TODO
    pub weight: usize,
    /// TODO
    pub long_term_weight: usize,
    /// TODO
    pub cumulative_difficulty: u128,
    /// TODO
    /// <https://github.com/Cuprate/cuprate/pull/102#discussion_r1556694072>
    /// <https://github.com/serai-dex/serai/blob/93be7a30674ecedfb325b6d09dc22d550d7c13f8/coins/monero/src/block.rs#L110>
    pub block_blob: Vec<u8>,
}

//---------------------------------------------------------------------------------------------------- OutputOnChain
/// An already approved previous transaction output.
#[derive(Clone, Debug, PartialEq, Eq)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))] // FIXME: monero_serai
// #[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub struct OutputOnChain {
    /// TODO
    pub height: u64,
    /// TODO
    pub time_lock: Timelock,
    /// TODO
    pub key: Option<EdwardsPoint>,
    /// TODO
    pub commitment: EdwardsPoint,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
