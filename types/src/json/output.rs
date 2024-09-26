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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[serde(untagged)]
pub enum Target {
    Key { key: String },
    TaggedKey { tagged_key: TaggedKey },
}

impl Default for Target {
    fn default() -> Self {
        Self::Key {
            key: Default::default(),
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TaggedKey {
    pub key: String,
    pub view_tag: String,
}
