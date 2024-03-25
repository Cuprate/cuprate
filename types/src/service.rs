//! Read/write `Request`s to the database.
//!
//! TODO: could add `strum` derives.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use monero_serai::{block::Block, transaction::Transaction};

#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::{ExtendedBlockHeader, OutputOnChain, VerifiedBlockInformation};

//---------------------------------------------------------------------------------------------------- ReadRequest
/// A read request to the database.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub enum ReadRequest {
    /// TODO
    BlockExtendedHeader(u64),
    /// TODO
    BlockHash(u64),
    /// TODO
    BlockExtendedHeaderInRange(Range<u64>),
    /// TODO
    ChainHeight,
    /// TODO
    GeneratedCoins,
    /// TODO
    Outputs(HashMap<u64, HashSet<u64>>),
    /// TODO
    NumberOutputsWithAmount(Vec<u64>),
    /// TODO
    CheckKIsNotSpent(HashSet<[u8; 32]>),
    /// TODO
    BlockBatchInRange(Range<u64>),
}

//---------------------------------------------------------------------------------------------------- WriteRequest
/// A write request to the database.
#[derive(Debug, Clone, PartialEq, Eq)]
// #[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub enum WriteRequest {
    /// TODO
    WriteBlock(VerifiedBlockInformation),
}

//---------------------------------------------------------------------------------------------------- Response
/// A response from the database.
///
/// TODO
#[derive(Debug, Clone, PartialEq, Eq)]
// #[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub enum Response {
    //------------------------------------------------------ Reads
    /// TODO
    BlockExtendedHeader(ExtendedBlockHeader),
    /// TODO
    BlockHash([u8; 32]),
    /// TODO
    BlockExtendedHeaderInRange(Vec<ExtendedBlockHeader>),
    /// TODO
    ChainHeight(u64, [u8; 32]),
    /// TODO
    GeneratedCoins(u64),
    /// TODO
    Outputs(HashMap<u64, HashMap<u64, OutputOnChain>>),
    /// TODO
    NumberOutputsWithAmount(HashMap<u64, usize>),
    /// TODO
    /// returns true if key images are spent
    CheckKIsNotSpent(bool),
    /// TODO
    BlockBatchInRange(Vec<(Block, Vec<Transaction>)>),

    //------------------------------------------------------ Writes
    /// TODO
    WriteBlockOk,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
