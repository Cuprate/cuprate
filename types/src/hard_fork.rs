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

/// An error working with a [`HardFork`].
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
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    ///
    /// # Errors
    ///
    /// Will return [`Err`] if the version is not a valid [`HardFork`].
    #[inline]
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

    /// Returns the hard-fork for a blocks [`BlockHeader::hardfork_signal`] (vote) field.
    ///
    /// <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    #[inline]
    pub fn from_vote(vote: u8) -> Self {
        if vote == 0 {
            // A vote of 0 is interpreted as 1 as that's what Monero used to default to.
            return Self::V1;
        }
        // This must default to the latest hard-fork!
        Self::from_version(vote).unwrap_or(Self::V16)
    }

    /// Returns the [`HardFork`] version and vote from this block header.
    ///
    /// # Errors
    ///
    /// Will return [`Err`] if the [`BlockHeader::hardfork_version`] is not a valid [`HardFork`].
    #[inline]
    pub fn from_block_header(header: &BlockHeader) -> Result<(Self, Self), HardForkError> {
        Ok((
            Self::from_version(header.hardfork_version)?,
            Self::from_vote(header.hardfork_signal),
        ))
    }

    /// Returns the raw hard-fork value, as it would appear in [`BlockHeader::hardfork_version`].
    pub const fn as_u8(&self) -> u8 {
        *self as u8
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
}
