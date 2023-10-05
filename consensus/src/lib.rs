pub mod block;
pub mod genesis;
pub mod hardforks;
pub mod miner_tx;
#[cfg(feature = "binaries")]
pub mod rpc;
pub mod verifier;

#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
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

    #[cfg(feature = "binaries")]
    BlockBatchInRange(std::ops::Range<u64>),
}

#[derive(Debug)]
pub enum DatabaseResponse {
    BlockHFInfo(hardforks::BlockHFInfo),
    BlockPOWInfo(block::pow::BlockPOWInfo),
    BlockWeights(block::weight::BlockWeightInfo),
    BlockHash([u8; 32]),

    BlockHfInfoInRange(Vec<hardforks::BlockHFInfo>),
    BlockWeightsInRange(Vec<block::weight::BlockWeightInfo>),
    BlockPOWInfoInRange(Vec<block::pow::BlockPOWInfo>),

    ChainHeight(u64),

    #[cfg(feature = "binaries")]
    BlockBatchInRange(
        Vec<(
            monero_serai::block::Block,
            Vec<monero_serai::transaction::Transaction>,
        )>,
    ),
}
