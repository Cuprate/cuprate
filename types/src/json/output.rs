//! JSON output types.
//!
//! The same [`Output`] is used in both
//! [`crate::json::block::MinerTransactionPrefix::vout`]
//! and [`crate::json::tx::TransactionPrefix::vout`].

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::hex::Hex;

/// JSON representation of an output.
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Output {
    pub amount: u64,
    pub target: Target,
}

/// [`Output::target`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Target {
    Key { key: Hex<32> },
    TaggedKey { tagged_key: TaggedKey },
}

impl Default for Target {
    fn default() -> Self {
        Self::Key {
            key: Default::default(),
        }
    }
}

/// [`Target::TaggedKey::tagged_key`].
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TaggedKey {
    pub key: Hex<32>,
    pub view_tag: Hex<1>,
}
