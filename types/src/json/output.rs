//! JSON output types.
//!
//! The same [`Output`] is used in both
//! [`crate::json::block::MinerTransaction::vout`] and [`crate::json::tx::Transaction::vout`].

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// JSON representation of an output.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Output {
    pub amount: u64,
    pub target: Target,
}

#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Target {
    /// Should be [`None`] if [`Self::tagged_key`] is [`Some`]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// Should be [`None`] if [`Self::key`] is [`Some`]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tagged_key: Option<TaggedKey>,
}

#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TaggedKey {
    pub key: String,
    pub view_tag: String,
}
