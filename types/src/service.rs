//! Database [`ReadRequest`]s, [`WriteRequest`]s, and [`Response`]s.
//!
//!
//! See [`cuprate_database`](https://github.com/Cuprate/cuprate/blob/00c3692eac6b2669e74cfd8c9b41c7e704c779ad/database/src/service/mod.rs#L1-L59)'s `service` module for more usage/documentation.

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
/// A read request to the database.
///
/// This pairs with [`Response`], where each variant here
/// matches in name with a `Response` variant. For example,
/// the proper response for a [`ReadRequest::BlockHash`]
/// would be a [`Response::BlockHash`].
///
/// See `Response` for the expected responses per `Request`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub enum ReadRequest {
    /// Request a block's extended header.
    ///
    /// The input is the block's height.
    BlockExtendedHeader(u64),

    /// Request a block's hash.
    ///
    /// The input is the block's height.
    BlockHash(u64),

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

    /// Check that all key images within a set arer not spent.
    ///
    /// Input is a set of key images.
    CheckKIsNotSpent(HashSet<[u8; 32]>),
}

//---------------------------------------------------------------------------------------------------- WriteRequest
/// A write request to the database.
///
/// There is currently only 1 write request to the database,
/// as such, the only valid [`Response`] to this request is
/// the proper response for a [`Response::WriteBlockOk`].
#[derive(Debug, Clone, PartialEq, Eq)]
// #[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub enum WriteRequest {
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
/// This pairs with [`ReadRequest`] and [`WriteRequest`],
/// see those two for more info.
#[derive(Debug, Clone, PartialEq, Eq)]
// #[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub enum Response {
    //------------------------------------------------------ Reads
    /// Response to [`ReadRequest::BlockExtendedHeader`].
    ///
    /// Inner value is the extended headed of the requested block.
    BlockExtendedHeader(ExtendedBlockHeader),

    /// Response to [`ReadRequest::BlockHash`].
    ///
    /// Inner value is the hash of the requested block.
    BlockHash([u8; 32]),

    /// Response to [`ReadRequest::BlockExtendedHeaderInRange`].
    ///
    /// Inner value is the list of extended header(s) of the requested block(s).
    BlockExtendedHeaderInRange(Vec<ExtendedBlockHeader>),

    /// Response to [`ReadRequest::ChainHeight`].
    ///
    /// Inner value is the chain height, and the top block's hash.
    ChainHeight(u64, [u8; 32]),

    /// Response to [`ReadRequest::GeneratedCoins`].
    ///
    /// Inner value is the total amount of generated coins so far, in atomic units.
    GeneratedCoins(u64),

    /// Response to [`ReadRequest::Outputs`].
    ///
    /// Inner value is all the outputs requested,
    /// associated with their amount and amount index.
    Outputs(HashMap<u64, HashMap<u64, OutputOnChain>>),

    /// Response to [`ReadRequest::NumberOutputsWithAmount`].
    ///
    /// Inner value is a `HashMap` of all the outputs requested where:
    /// - Key = output amount
    /// - Value = count of outputs with the same amount
    NumberOutputsWithAmount(HashMap<u64, usize>),

    /// Response to [`ReadRequest::CheckKIsNotSpent`].
    ///
    /// The inner value is `true` if _any_ of the key images
    /// were spent (exited in the database already).
    ///
    /// The inner value is `false` if _none_ of the key images were spent.
    CheckKIsNotSpent(bool),

    //------------------------------------------------------ Writes
    /// Response to [`WriteRequest::WriteBlock`].
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
