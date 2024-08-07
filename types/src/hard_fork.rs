use std::time::Duration;

use monero_serai::block::BlockHeader;

/// Target block time for hf 1.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
const BLOCK_TIME_V1: Duration = Duration::from_secs(60);
/// Target block time from v2.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
const BLOCK_TIME_V2: Duration = Duration::from_secs(120);

/// The raw hard-fork value was not a valid [`HardFork`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HardForkError {
    /// The raw-HF value is not a valid [`HardFork`].
    #[error("The hard-fork is unknown")]
    HardForkUnknown,
    /// The [`HardFork`] version is incorrect.
    #[error("The block is on an incorrect hard-fork")]
    VersionIncorrect,
    /// The block's [`HardFork`] vote was below the current [`HardFork`].
    #[error("The block's vote is for a previous hard-fork")]
    VoteTooLow,
}

/// An identifier for every hard-fork Monero has had.
#[allow(missing_docs)]
#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
#[cfg_attr(any(feature = "proptest"), derive(proptest_derive::Arbitrary))]
#[repr(u8)]
pub enum HardFork {
    #[default]
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
    /// Returns the hard-fork for a blocks [`BlockHeader::hardfork_version`] field.
    ///
    /// <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    #[inline]
    pub fn from_version(version: u8) -> Result<HardFork, HardForkError> {
        Ok(match version {
            1 => HardFork::V1,
            2 => HardFork::V2,
            3 => HardFork::V3,
            4 => HardFork::V4,
            5 => HardFork::V5,
            6 => HardFork::V6,
            7 => HardFork::V7,
            8 => HardFork::V8,
            9 => HardFork::V9,
            10 => HardFork::V10,
            11 => HardFork::V11,
            12 => HardFork::V12,
            13 => HardFork::V13,
            14 => HardFork::V14,
            15 => HardFork::V15,
            16 => HardFork::V16,
            _ => return Err(HardForkError::HardForkUnknown),
        })
    }

    /// Returns the hard-fork for a blocks [`BlockHeader::hardfork_signal`] (vote) field.
    ///
    /// <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    #[inline]
    pub fn from_vote(vote: u8) -> HardFork {
        if vote == 0 {
            // A vote of 0 is interpreted as 1 as that's what Monero used to default to.
            return HardFork::V1;
        }
        // This must default to the latest hard-fork!
        Self::from_version(vote).unwrap_or(HardFork::V16)
    }

    /// Returns the [`HardFork`] version and vote from this block header.
    #[inline]
    pub fn from_block_header(header: &BlockHeader) -> Result<(HardFork, HardFork), HardForkError> {
        Ok((
            HardFork::from_version(header.hardfork_version)?,
            HardFork::from_vote(header.hardfork_signal),
        ))
    }

    /// Returns the next hard-fork.
    pub fn next_fork(&self) -> Option<HardFork> {
        HardFork::from_version(*self as u8 + 1).ok()
    }

    /// Returns the target block time for this hardfork.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
    pub fn block_time(&self) -> Duration {
        match self {
            HardFork::V1 => BLOCK_TIME_V1,
            _ => BLOCK_TIME_V2,
        }
    }
}