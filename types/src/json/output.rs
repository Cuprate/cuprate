//! JSON output types.
//!
//! The same [`Output`] is used in both
//! [`crate::json::block::MinerTransactionPrefix::vout`]
//! and [`crate::json::tx::TransactionPrefix::vout`].

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::hex::HexBytes;

/// JSON representation of an output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Output {
    pub amount: u64,
    pub target: Target,
}

/// [`Output::target`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Target {
    Key { key: HexBytes<32> },
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
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TaggedKey {
    pub key: HexBytes<32>,
    pub view_tag: HexBytes<1>,
}
