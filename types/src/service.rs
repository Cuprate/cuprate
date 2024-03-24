//! Read/write `Request`s to the database.
//!
//! TODO: could add `strum` derives.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

//---------------------------------------------------------------------------------------------------- ReadRequest
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A read request to the database.
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
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A write request to the database.
pub enum WriteRequest {
    /// TODO
    WriteBlock(VerifiedBlockInformation),
}

//---------------------------------------------------------------------------------------------------- Response
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A response from the database.
///
/// TODO
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
    BlockBatchInRange(
        Vec<(
            monero_serai::block::Block,
            Vec<monero_serai::transaction::Transaction>,
        )>,
    ),

    //------------------------------------------------------ Writes
    /// TODO
    WriteBlockOk,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
