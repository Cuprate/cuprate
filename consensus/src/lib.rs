pub mod genesis;
pub mod hardforks;
pub mod pow;
#[cfg(feature = "rpc")]
pub mod rpc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid hard fork version: {0}")]
    InvalidHardForkVersion(&'static str),
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
    BlockHeader(cuprate_common::BlockID),
    BlockPOWInfo(cuprate_common::BlockID),
    ChainHeight,
}

#[derive(Debug)]
pub enum DatabaseResponse {
    BlockHeader(monero_serai::block::BlockHeader),
    BlockPOWInfo(pow::BlockPOWInfo),
    ChainHeight(u64),
}
