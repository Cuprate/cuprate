//! Database [`BCReadRequest`]s, [`BCWriteRequest`]s, and [`BCResponse`]s.
//!
//! Tests that assert particular requests lead to particular
//! responses are also tested in Cuprate's blockchain database crate.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::{ExtendedBlockHeader, OutputOnChain, VerifiedBlockInformation};

//---------------------------------------------------------------------------------------------------- ReadRequest
/// A read request to the blockchain database.
///
/// This pairs with [`BCResponse`], where each variant here
/// matches in name with a [`BCResponse`] variant. For example,
/// the proper response for a [`BCReadRequest::BlockHash`]
/// would be a [`BCResponse::BlockHash`].
///
/// See `Response` for the expected responses per `Request`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BCReadRequest {
    /// Request a block's extended header.
    ///
    /// The input is the block's height.
    BlockExtendedHeader(u64),

    /// Request a block's hash.
    ///
    /// The input is the block's height.
    BlockHash(u64),

    /// Removes the block hashes that are not in the _main_ chain.
    ///
    /// This should filter (remove) hashes in alt-blocks as well.
    FilterUnknownHashes(HashSet<[u8; 32]>),

    /// Request a range of block extended headers.
    ///
    /// The input is a range of block heights.
    BlockExtendedHeaderInRange(Range<u64>),

    /// Request the current chain height.
    ///
    /// Note that this is not the top-block height.
    ChainHeight,

    /// Request the total amount of generated coins (atomic units) so far.
    GeneratedCoins,

    /// Request data for multiple outputs.
    ///
    /// The input is a `HashMap` where:
    /// - Key = output amount
    /// - Value = set of amount indices
    ///
    /// For pre-RCT outputs, the amount is non-zero,
    /// and the amount indices represent the wanted
    /// indices of duplicate amount outputs, i.e.:
    ///
    /// ```ignore
    /// // list of outputs with amount 10
    /// [0, 1, 2, 3, 4, 5]
    /// //  ^     ^
    /// // we only want these two, so we would provide
    /// // `amount: 10, amount_index: {1, 3}`
    /// ```
    ///
    /// For RCT outputs, the amounts would be `0` and
    /// the amount indices would represent the global
    /// RCT output indices.
    Outputs(HashMap<u64, HashSet<u64>>),

    /// Request the amount of outputs with a certain amount.
    ///
    /// The input is a list of output amounts.
    NumberOutputsWithAmount(Vec<u64>),

    /// Check that all key images within a set are not spent.
    ///
    /// Input is a set of key images.
    KeyImagesSpent(HashSet<[u8; 32]>),

    /// A request for the compact chain history.
    CompactChainHistory,

    /// A request to find the first unknown block ID in a list of block IDs.
    ////
    /// # Invariant
    /// The [`Vec`] containing the block IDs must be sorted in chronological block
    /// order, or else the returned response is unspecified and meaningless,
    /// as this request performs a binary search.
    FindFirstUnknown(Vec<[u8; 32]>),
}

//---------------------------------------------------------------------------------------------------- WriteRequest
/// A write request to the blockchain database.
///
/// There is currently only 1 write request to the database,
/// as such, the only valid [`BCResponse`] to this request is
/// the proper response for a [`BCResponse::WriteBlockOk`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BCWriteRequest {
    /// Request that a block be written to the database.
    ///
    /// Input is an already verified block.
    WriteBlock(VerifiedBlockInformation),
}

//---------------------------------------------------------------------------------------------------- Response
/// A response from the database.
///
/// These are the data types returned when using sending a `Request`.
///
/// This pairs with [`BCReadRequest`] and [`BCWriteRequest`],
/// see those two for more info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BCResponse {
    //------------------------------------------------------ Reads
    /// Response to [`BCReadRequest::BlockExtendedHeader`].
    ///
    /// Inner value is the extended headed of the requested block.
    BlockExtendedHeader(ExtendedBlockHeader),

    /// Response to [`BCReadRequest::BlockHash`].
    ///
    /// Inner value is the hash of the requested block.
    BlockHash([u8; 32]),

    /// Response to [`BCReadRequest::FilterUnknownHashes`].
    ///
    /// Inner value is the list of hashes that were in the main chain.
    FilterUnknownHashes(HashSet<[u8; 32]>),

    /// Response to [`BCReadRequest::BlockExtendedHeaderInRange`].
    ///
    /// Inner value is the list of extended header(s) of the requested block(s).
    BlockExtendedHeaderInRange(Vec<ExtendedBlockHeader>),

    /// Response to [`BCReadRequest::ChainHeight`].
    ///
    /// Inner value is the chain height, and the top block's hash.
    ChainHeight(u64, [u8; 32]),

    /// Response to [`BCReadRequest::GeneratedCoins`].
    ///
    /// Inner value is the total amount of generated coins so far, in atomic units.
    GeneratedCoins(u64),

    /// Response to [`BCReadRequest::Outputs`].
    ///
    /// Inner value is all the outputs requested,
    /// associated with their amount and amount index.
    Outputs(HashMap<u64, HashMap<u64, OutputOnChain>>),

    /// Response to [`BCReadRequest::NumberOutputsWithAmount`].
    ///
    /// Inner value is a `HashMap` of all the outputs requested where:
    /// - Key = output amount
    /// - Value = count of outputs with the same amount
    NumberOutputsWithAmount(HashMap<u64, usize>),

    /// Response to [`BCReadRequest::KeyImagesSpent`].
    ///
    /// The inner value is `true` if _any_ of the key images
    /// were spent (existed in the database already).
    ///
    /// The inner value is `false` if _none_ of the key images were spent.
    KeyImagesSpent(bool),

    /// Response to [`BCReadRequest::CompactChainHistory`].
    CompactChainHistory {
        /// A list of blocks IDs in our chain, starting with the most recent block, all the way to the genesis block.
        ///
        /// These blocks should be in reverse chronological order, not every block is needed.
        block_ids: Vec<[u8; 32]>,
        /// The current cumulative difficulty of the chain.
        cumulative_difficulty: u128,
    },

    /// The response for [`BCReadRequest::FindFirstUnknown`].
    ///
    /// Contains the index of the first unknown block and its expected height.
    ///
    /// This will be [`None`] if all blocks were known.
    FindFirstUnknown(Option<(usize, u64)>),

    //------------------------------------------------------ Writes
    /// Response to [`BCWriteRequest::WriteBlock`].
    ///
    /// This response indicates that the requested block has
    /// successfully been written to the database without error.
    WriteBlockOk,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
