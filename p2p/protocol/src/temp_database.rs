use monero::Hash;
use monero_wire::messages::CoreSyncData;
use thiserror::Error;

pub enum BlockKnown {
    No,
    OnMainChain,
    OnSideChain,
    KnownBad,
}

impl BlockKnown {
    pub fn is_known(&self) -> bool {
        !matches!(self, BlockKnown::No)
    }
}

pub enum DataBaseRequest {
    CurrentHeight,
    CumulativeDifficulty,
    CoreSyncData,
    Chain,
    BlockHeight(Hash),
    BlockKnown(Hash),

}

pub enum DataBaseResponse {
    CurrentHeight(u64),
    CumulativeDifficulty(u128),
    CoreSyncData(CoreSyncData),
    Chain(Vec<Hash>),
    BlockHeight(Option<u64>),
    BlockKnown(BlockKnown)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DatabaseError {

}
