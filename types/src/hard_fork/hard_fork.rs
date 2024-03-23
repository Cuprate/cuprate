//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::VecDeque,
    fmt::{Display, Formatter},
    time::Duration,
};

use bytemuck::Contiguous;
use monero_serai::block::BlockHeader;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::hard_fork::{
    constants::{BLOCK_TIME_V1, BLOCK_TIME_V2},
    error::HardForkError,
};

//---------------------------------------------------------------------------------------------------- HardFork
/// An identifier for every hard-fork Monero has had.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Contiguous)]
// #[cfg_attr(any(feature = "proptest", test), derive(proptest_derive::Arbitrary))] // TODO: fix this
#[repr(u8)]
#[allow(missing_docs)]
pub enum HardFork {
    V1 = 1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13,
    V14,
    V15,
    // remember to update from_vote!
    V16,
}

impl HardFork {
    /// Returns the hard-fork for a blocks `major_version` field.
    ///
    /// <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    ///
    /// # Errors
    /// TODO
    pub const fn from_version(version: u8) -> Result<Self, HardForkError> {
        Ok(match version {
            1 => Self::V1,
            2 => Self::V2,
            3 => Self::V3,
            4 => Self::V4,
            5 => Self::V5,
            6 => Self::V6,
            7 => Self::V7,
            8 => Self::V8,
            9 => Self::V9,
            10 => Self::V10,
            11 => Self::V11,
            12 => Self::V12,
            13 => Self::V13,
            14 => Self::V14,
            15 => Self::V15,
            16 => Self::V16,
            _ => return Err(HardForkError::HardForkUnknown),
        })
    }

    /// Returns the hard-fork for a blocks `minor_version` (vote) field.
    ///
    /// <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    pub fn from_vote(vote: u8) -> Self {
        if vote == 0 {
            // A vote of 0 is interpreted as 1 as that's what Monero used to default to.
            return Self::V1;
        }
        // This must default to the latest hard-fork!
        Self::from_version(vote).unwrap_or(Self::V16)
    }

    /// TODO
    ///
    /// # Errors
    /// TODO
    pub fn from_block_header(header: &BlockHeader) -> Result<(Self, Self), HardForkError> {
        Ok((
            Self::from_version(header.major_version)?,
            Self::from_vote(header.minor_version),
        ))
    }

    /// Returns the next hard-fork.
    pub fn next_fork(&self) -> Option<Self> {
        Self::from_version(*self as u8 + 1).ok()
    }

    /// Returns the target block time for this hardfork.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
    pub const fn block_time(&self) -> Duration {
        match self {
            Self::V1 => BLOCK_TIME_V1,
            _ => BLOCK_TIME_V2,
        }
    }

    /// Checks a blocks version and vote, assuming that `self` is the current hard-fork.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    ///
    /// # Errors
    /// TODO
    pub fn check_block_version_vote(
        &self,
        version: &Self,
        vote: &Self,
    ) -> Result<(), HardForkError> {
        // self = current hf
        if self != version {
            return Err(HardForkError::VersionIncorrect);
        }
        if self > vote {
            return Err(HardForkError::VoteTooLow);
        }

        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
