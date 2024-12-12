//! TODO

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    error,
    macros::bytes::{Buf, BufMut},
    EpeeObject, EpeeObjectBuilder,
};

use cuprate_hex::Hex;

#[cfg(feature = "serde")]
use crate::serde::{serde_false, serde_true};

//---------------------------------------------------------------------------------------------------- TxEntry
#[doc = crate::macros::monero_definition_link!(
    "cc73fe71162d564ffda8e549b79a350bca53c454",
    "rpc/core_rpc_server_commands_defs.h",
    389..=428
)]
/// Used in [`crate::other::GetTransactionsResponse`].
///
/// # Epee
/// This type is only used in a JSON endpoint, so the
/// epee implementation on this type only panics.
///
/// It is only implemented to satisfy the RPC type generator
/// macro, which requires all objects to be serde + epee.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TxEntry {
    /// `cuprate_types::json::tx::Transaction` should be used
    /// to create this JSON string in a type-safe manner.
    pub as_json: String,

    pub as_hex: String,
    pub double_spend_seen: bool,
    pub tx_hash: Hex<32>,
    pub prunable_as_hex: String,
    pub prunable_hash: Hex<32>,
    pub pruned_as_hex: String,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub tx_entry_type: TxEntryType,
}

/// Different fields in [`TxEntry`] variants.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum TxEntryType {
    /// This transaction exists in the blockchain.
    Blockchain {
        block_height: u64,
        block_timestamp: u64,
        confirmations: u64,
        output_indices: Vec<u64>,

        /// Will always be serialized as `false`.
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_false"))]
        in_pool: bool,
    },

    /// This transaction exists in the transaction pool.
    Pool {
        received_timestamp: u64,
        relayed: bool,

        /// Will always be serialized as `true`.
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_true"))]
        in_pool: bool,
    },
}

impl Default for TxEntryType {
    fn default() -> Self {
        Self::Blockchain {
            block_height: Default::default(),
            block_timestamp: Default::default(),
            confirmations: Default::default(),
            output_indices: Default::default(),
            in_pool: Default::default(),
        }
    }
}
//---------------------------------------------------------------------------------------------------- Epee
#[cfg(feature = "epee")]
impl EpeeObjectBuilder<TxEntry> for () {
    fn add_field<B: Buf>(&mut self, _: &str, _: &mut B) -> error::Result<bool> {
        unreachable!()
    }

    fn finish(self) -> error::Result<TxEntry> {
        unreachable!()
    }
}

#[cfg(feature = "epee")]
impl EpeeObject for TxEntry {
    type Builder = ();

    fn number_of_fields(&self) -> u64 {
        unreachable!()
    }

    fn write_fields<B: BufMut>(self, _: &mut B) -> error::Result<()> {
        unreachable!()
    }
}
