//! The [`HardFork`] type.
use std::time::Duration;

use strum::{
    AsRefStr, Display, EnumCount, EnumIs, EnumString, FromRepr, IntoStaticStr, VariantArray,
};

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
#[derive(
    Default,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Copy,
    Clone,
    Hash,
    EnumCount,
    Display,
    AsRefStr,
    EnumIs,
    EnumString,
    FromRepr,
    IntoStaticStr,
    VariantArray,
)]
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
    V16,
}

impl HardFork {
    /// The latest [`HardFork`].
    ///
    /// ```rust
    /// # use cuprate_types::HardFork;
    /// assert_eq!(HardFork::LATEST, HardFork::V16);
    /// ```
    pub const LATEST: Self = Self::VARIANTS[Self::COUNT - 1];

    /// Returns the hard-fork for a blocks [`BlockHeader::hardfork_version`] field.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    ///
    /// # Errors
    /// Will return [`Err`] if the version is not a valid [`HardFork`].
    ///
    /// ```rust
    /// # use cuprate_types::{HardFork, HardForkError};
    /// # use strum::VariantArray;
    /// assert_eq!(HardFork::from_version(0), Err(HardForkError::HardForkUnknown));
    /// assert_eq!(HardFork::from_version(17), Err(HardForkError::HardForkUnknown));
    ///
    /// for (version, hf) in HardFork::VARIANTS.iter().enumerate() {
    ///     // +1 because enumerate starts at 0, hf starts at 1.
    ///     assert_eq!(*hf, HardFork::from_version(version as u8 + 1).unwrap());
    /// }
    /// ```
    #[inline]
    pub const fn from_version(version: u8) -> Result<Self, HardForkError> {
        match Self::from_repr(version) {
            Some(this) => Ok(this),
            None => Err(HardForkError::HardForkUnknown),
        }
    }

    /// Returns the hard-fork for a blocks [`BlockHeader::hardfork_signal`] (vote) field.
    ///
    /// <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    ///
    /// ```rust
    /// # use cuprate_types::{HardFork, HardForkError};
    /// # use strum::VariantArray;
    /// // 0 is interpreted as 1.
    /// assert_eq!(HardFork::from_vote(0), HardFork::V1);
    /// // Unknown defaults to `LATEST`.
    /// assert_eq!(HardFork::from_vote(17), HardFork::V16);
    ///
    /// for (vote, hf) in HardFork::VARIANTS.iter().enumerate() {
    ///     // +1 because enumerate starts at 0, hf starts at 1.
    ///     assert_eq!(*hf, HardFork::from_vote(vote as u8 + 1));
    /// }
    /// ```
    #[inline]
    pub fn from_vote(vote: u8) -> Self {
        if vote == 0 {
            // A vote of 0 is interpreted as 1 as that's what Monero used to default to.
            Self::V1
        } else {
            // This must default to the latest hard-fork!
            Self::from_version(vote).unwrap_or(Self::LATEST)
        }
    }

    /// Returns the [`HardFork`] version and vote from this block header.
    ///
    /// # Errors
    /// Will return [`Err`] if the [`BlockHeader::hardfork_version`] is not a valid [`HardFork`].
    #[inline]
    pub fn from_block_header(header: &BlockHeader) -> Result<(Self, Self), HardForkError> {
        Ok((
            Self::from_version(header.hardfork_version)?,
            Self::from_vote(header.hardfork_signal),
        ))
    }

    /// Returns the raw hard-fork value, as it would appear in [`BlockHeader::hardfork_version`].
    ///
    /// ```rust
    /// # use cuprate_types::{HardFork, HardForkError};
    /// # use strum::VariantArray;
    /// for (i, hf) in HardFork::VARIANTS.iter().enumerate() {
    ///     // +1 because enumerate starts at 0, hf starts at 1.
    ///     assert_eq!(hf.as_u8(), i as u8 + 1);
    /// }
    /// ```
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns the next hard-fork.
    pub fn next_fork(self) -> Option<Self> {
        Self::from_version(self as u8 + 1).ok()
    }

    /// Returns the target block time for this hardfork.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
    pub const fn block_time(self) -> Duration {
        match self {
            Self::V1 => BLOCK_TIME_V1,
            _ => BLOCK_TIME_V2,
        }
    }

    /// Returns `true` if `self` is [`Self::LATEST`].
    ///
    /// ```rust
    /// # use cuprate_types::HardFork;
    /// # use strum::VariantArray;
    ///
    /// for hf in HardFork::VARIANTS.iter() {
    ///     if *hf == HardFork::LATEST {
    ///         assert!(hf.is_latest());
    ///     } else {
    ///         assert!(!hf.is_latest());
    ///     }
    /// }
    /// ```
    pub const fn is_latest(self) -> bool {
        matches!(self, Self::LATEST)
    }
}
