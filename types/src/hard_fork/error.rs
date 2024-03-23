//! TODO

//---------------------------------------------------------------------------------------------------- Import
use bytemuck::{AnyBitPattern, NoUninit, Pod, Zeroable};

use monero_serai::block::BlockHeader;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

//---------------------------------------------------------------------------------------------------- Import
/// TODO
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, thiserror::Error)]
pub enum HardForkError {
    /// TODO
    #[error("The hard-fork is unknown")]
    HardForkUnknown,

    /// TODO
    #[error("The block is on an incorrect hard-fork")]
    VersionIncorrect,

    /// TODO
    #[error("The block's vote is for a previous hard-fork")]
    VoteTooLow,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
