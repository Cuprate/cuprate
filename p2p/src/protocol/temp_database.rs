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
    BlockHeight([u8; 32]),
    BlockKnown([u8; 32]),
}

pub enum DataBaseResponse {
    CurrentHeight(u64),
    CumulativeDifficulty(u128),
    CoreSyncData(CoreSyncData),
    Chain(Vec<[u8; 32]>),
    BlockHeight(Option<u64>),
    BlockKnown(BlockKnown),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DatabaseError {}
