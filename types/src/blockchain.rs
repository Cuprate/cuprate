//! Database [`BlockchainReadRequest`]s, [`BlockchainWriteRequest`]s, and [`BlockchainResponse`]s.
//!
//! Tests that assert particular requests lead to particular
//! responses are also tested in Cuprate's blockchain database crate.
//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use monero_serai::block::Block;

use crate::{
    types::{Chain, ExtendedBlockHeader, OutputOnChain, VerifiedBlockInformation},
    AltBlockInformation, ChainId, CoinbaseTxSum, OutputHistogramEntry, OutputHistogramInput,
};

//---------------------------------------------------------------------------------------------------- ReadRequest
/// A read request to the blockchain database.
///
/// This pairs with [`BlockchainResponse`], where each variant here
/// matches in name with a [`BlockchainResponse`] variant. For example,
/// the proper response for a [`BlockchainReadRequest::BlockHash`]
/// would be a [`BlockchainResponse::BlockHash`].
///
/// See `Response` for the expected responses per `Request`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockchainReadRequest {
    /// Request a block's extended header.
    ///
    /// The input is the block's height.
    BlockExtendedHeader(usize),

    /// Request a block's hash.
    ///
    /// The input is the block's height and the chain it is on.
    BlockHash(usize, Chain),

    /// Request to check if we have a block and which [`Chain`] it is on.
    ///
    /// The input is the block's hash.
    FindBlock([u8; 32]),

    /// Removes the block hashes that are not in the _main_ chain.
    ///
    /// This should filter (remove) hashes in alt-blocks as well.
    FilterUnknownHashes(HashSet<[u8; 32]>),

    /// Request a range of block extended headers.
    ///
    /// The input is a range of block heights.
    BlockExtendedHeaderInRange(Range<usize>, Chain),

    /// Request the current chain height.
    ///
    /// Note that this is not the top-block height.
    ChainHeight,

    /// Request the total amount of generated coins (atomic units) at this height.
    GeneratedCoins(usize),

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
    ///
    /// # Invariant
    /// The [`Vec`] containing the block IDs must be sorted in chronological block
    /// order, or else the returned response is unspecified and meaningless,
    /// as this request performs a binary search.
    FindFirstUnknown(Vec<[u8; 32]>),

    /// A request for all alt blocks in the chain with the given [`ChainId`].
    AltBlocksInChain(ChainId),

    /// Get a [`Block`] by its height.
    Block { height: usize },

    /// Get a [`Block`] by its hash.
    BlockByHash([u8; 32]),

    /// Get the total amount of non-coinbase transactions in the chain.
    TotalTxCount,

    /// Get the current size of the database.
    DatabaseSize,

    /// Get an output histogram.
    ///
    /// TODO: document fields after impl.
    OutputHistogram(OutputHistogramInput),

    /// Get the coinbase amount and the fees amount for
    /// `N` last blocks starting at particular height.
    ///
    /// TODO: document fields after impl.
    CoinbaseTxSum { height: usize, count: u64 },
}

//---------------------------------------------------------------------------------------------------- WriteRequest
/// A write request to the blockchain database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockchainWriteRequest {
    /// Request that a block be written to the database.
    ///
    /// Input is an already verified block.
    WriteBlock(VerifiedBlockInformation),

    /// Write an alternative block to the database,
    ///
    /// Input is the alternative block.
    WriteAltBlock(AltBlockInformation),

    /// A request to pop some blocks from the top of the main chain
    ///
    /// Input is the amount of blocks to pop.
    ///
    /// This request flushes all alt-chains from the cache before adding the popped blocks to the
    /// alt cache.
    PopBlocks(usize),

    /// A request to reverse the re-org process.
    ///
    /// The inner value is the [`ChainId`] of the old main chain.
    ///
    /// # Invariant
    /// It is invalid to call this with a [`ChainId`] that was not returned from [`BlockchainWriteRequest::PopBlocks`].
    ReverseReorg(ChainId),

    /// A request to flush all alternative blocks.
    FlushAltBlocks,
}

//---------------------------------------------------------------------------------------------------- Response
/// A response from the database.
///
/// These are the data types returned when using sending a `Request`.
///
/// This pairs with [`BlockchainReadRequest`] and [`BlockchainWriteRequest`],
/// see those two for more info.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(clippy::large_enum_variant)]
pub enum BlockchainResponse {
    //------------------------------------------------------ Reads
    /// Response to [`BlockchainReadRequest::BlockExtendedHeader`].
    ///
    /// Inner value is the extended headed of the requested block.
    BlockExtendedHeader(ExtendedBlockHeader),

    /// Response to [`BlockchainReadRequest::BlockHash`].
    ///
    /// Inner value is the hash of the requested block.
    BlockHash([u8; 32]),

    /// Response to [`BlockchainReadRequest::FindBlock`].
    ///
    /// Inner value is the chain and height of the block if found.
    FindBlock(Option<(Chain, usize)>),

    /// Response to [`BlockchainReadRequest::FilterUnknownHashes`].
    ///
    /// Inner value is the list of hashes that were in the main chain.
    FilterUnknownHashes(HashSet<[u8; 32]>),

    /// Response to [`BlockchainReadRequest::BlockExtendedHeaderInRange`].
    ///
    /// Inner value is the list of extended header(s) of the requested block(s).
    BlockExtendedHeaderInRange(Vec<ExtendedBlockHeader>),

    /// Response to [`BlockchainReadRequest::ChainHeight`].
    ///
    /// Inner value is the chain height, and the top block's hash.
    ChainHeight(usize, [u8; 32]),

    /// Response to [`BlockchainReadRequest::GeneratedCoins`].
    ///
    /// Inner value is the total amount of generated coins up to and including the chosen height, in atomic units.
    GeneratedCoins(u64),

    /// Response to [`BlockchainReadRequest::Outputs`].
    ///
    /// Inner value is all the outputs requested,
    /// associated with their amount and amount index.
    Outputs(HashMap<u64, HashMap<u64, OutputOnChain>>),

    /// Response to [`BlockchainReadRequest::NumberOutputsWithAmount`].
    ///
    /// Inner value is a `HashMap` of all the outputs requested where:
    /// - Key = output amount
    /// - Value = count of outputs with the same amount
    NumberOutputsWithAmount(HashMap<u64, usize>),

    /// Response to [`BlockchainReadRequest::KeyImagesSpent`].
    ///
    /// The inner value is `true` if _any_ of the key images
    /// were spent (existed in the database already).
    ///
    /// The inner value is `false` if _none_ of the key images were spent.
    KeyImagesSpent(bool),

    /// Response to [`BlockchainReadRequest::CompactChainHistory`].
    CompactChainHistory {
        /// A list of blocks IDs in our chain, starting with the most recent block, all the way to the genesis block.
        ///
        /// These blocks should be in reverse chronological order, not every block is needed.
        block_ids: Vec<[u8; 32]>,
        /// The current cumulative difficulty of the chain.
        cumulative_difficulty: u128,
    },

    /// Response to [`BlockchainReadRequest::FindFirstUnknown`].
    ///
    /// Contains the index of the first unknown block and its expected height.
    ///
    /// This will be [`None`] if all blocks were known.
    FindFirstUnknown(Option<(usize, usize)>),

    /// Response to [`BlockchainReadRequest::AltBlocksInChain`].
    ///
    /// Contains all the alt blocks in the alt-chain in chronological order.
    AltBlocksInChain(Vec<AltBlockInformation>),

    /// Response to:
    /// - [`BlockchainReadRequest::Block`].
    /// - [`BlockchainReadRequest::BlockByHash`].
    Block(Block),

    /// Response to [`BlockchainReadRequest::TotalTxCount`].
    TotalTxCount(usize),

    /// Response to [`BlockchainReadRequest::DatabaseSize`].
    DatabaseSize {
        /// The size of the database file in bytes.
        database_size: u64,
        /// The amount of free bytes there are
        /// the disk where the database is located.
        free_space: u64,
    },

    /// Response to [`BlockchainReadRequest::OutputHistogram`].
    OutputHistogram(Vec<OutputHistogramEntry>),

    /// Response to [`BlockchainReadRequest::CoinbaseTxSum`].
    CoinbaseTxSum(CoinbaseTxSum),

    //------------------------------------------------------ Writes
    /// A generic Ok response to indicate a request was successfully handled.
    ///
    /// currently the response for:
    /// - [`BlockchainWriteRequest::WriteBlock`]
    /// - [`BlockchainWriteRequest::WriteAltBlock`]
    /// - [`BlockchainWriteRequest::ReverseReorg`]
    /// - [`BlockchainWriteRequest::FlushAltBlocks`]
    Ok,

    /// Response to [`BlockchainWriteRequest::PopBlocks`].
    ///
    /// The inner value is the alt-chain ID for the old main chain blocks.
    PopBlocks(ChainId),
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
