use std::collections::{HashMap, HashSet};

pub mod block;
pub mod genesis;
pub mod hardforks;
mod helper;
pub mod miner_tx;
#[cfg(feature = "binaries")]
pub mod rpc;
pub mod transactions;
pub mod verifier;

#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Transaction sig invalid: {0}")]
    TransactionSignatureInvalid(&'static str),
    #[error("Transaction inputs overflow")]
    TransactionInputsOverflow,
    #[error("Transaction outputs overflow")]
    TransactionOutputsOverflow,
    #[error("Transaction has an invalid output: {0}")]
    TransactionInvalidOutput(&'static str),
    #[error("Transaction has an invalid version")]
    TransactionVersionInvalid,
    #[error("Transaction an invalid input: {0}")]
    TransactionHasInvalidInput(&'static str),
    #[error("Transaction has invalid ring: {0}")]
    TransactionHasInvalidRing(&'static str),
    #[error("Block has an invalid proof of work")]
    BlockPOWInvalid,
    #[error("Block has a timestamp outside of the valid range")]
    BlockTimestampInvalid,
    #[error("Block is too large")]
    BlockIsTooLarge,
    #[error("Invalid hard fork version: {0}")]
    InvalidHardForkVersion(&'static str),
    #[error("The block has a different previous hash than expected")]
    BlockIsNotApartOfChain,
    #[error("Database error: {0}")]
    Database(#[from] tower::BoxError),
}

pub trait Database:
    tower::Service<DatabaseRequest, Response = DatabaseResponse, Error = tower::BoxError>
{
}

impl<T: tower::Service<DatabaseRequest, Response = DatabaseResponse, Error = tower::BoxError>>
    Database for T
{
}

#[derive(Debug, Clone)]
pub enum DatabaseRequest {
    BlockHFInfo(cuprate_common::BlockID),
    BlockPOWInfo(cuprate_common::BlockID),
    BlockWeights(cuprate_common::BlockID),
    BlockHash(u64),

    BlockHfInfoInRange(std::ops::Range<u64>),
    BlockWeightsInRange(std::ops::Range<u64>),
    BlockPOWInfoInRange(std::ops::Range<u64>),

    ChainHeight,

    Outputs(HashMap<u64, HashSet<u64>>),
    NumberOutputsWithAmount(u64),

    #[cfg(feature = "binaries")]
    BlockBatchInRange(std::ops::Range<u64>),
}

#[derive(Debug)]
pub enum DatabaseResponse {
    BlockHFInfo(hardforks::BlockHFInfo),
    BlockPOWInfo(block::BlockPOWInfo),
    BlockWeights(block::weight::BlockWeightInfo),
    BlockHash([u8; 32]),

    BlockHfInfoInRange(Vec<hardforks::BlockHFInfo>),
    BlockWeightsInRange(Vec<block::BlockWeightInfo>),
    BlockPOWInfoInRange(Vec<block::BlockPOWInfo>),

    ChainHeight(u64),

    Outputs(HashMap<u64, HashMap<u64, [curve25519_dalek::EdwardsPoint; 2]>>),
    NumberOutputsWithAmount(usize),

    #[cfg(feature = "binaries")]
    BlockBatchInRange(
        Vec<(
            monero_serai::block::Block,
            Vec<monero_serai::transaction::Transaction>,
        )>,
    ),
}
