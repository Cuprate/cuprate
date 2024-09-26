//! TODO

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use crate::serde::{serde_false, serde_true};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    error,
    macros::bytes::{Buf, BufMut},
    EpeeObject, EpeeObjectBuilder,
};

//---------------------------------------------------------------------------------------------------- TxEntry
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
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
///
/// # Example
/// ```rust
/// use cuprate_rpc_types::misc::*;
/// use serde_json::{json, from_value};
///
/// let json = json!({
///     "as_hex": String::default(),
///     "as_json": String::default(),
///     "block_height": u64::default(),
///     "block_timestamp": u64::default(),
///     "confirmations": u64::default(),
///     "double_spend_seen": bool::default(),
///     "output_indices": Vec::<u64>::default(),
///     "prunable_as_hex": String::default(),
///     "prunable_hash": String::default(),
///     "pruned_as_hex": String::default(),
///     "tx_hash": String::default(),
///     "in_pool": bool::default(),
/// });
/// let tx_entry = from_value::<TxEntry>(json).unwrap();
/// assert!(matches!(tx_entry, TxEntry::InPool { .. }));
///
/// let json = json!({
///     "as_hex": String::default(),
///     "as_json": String::default(),
///     "double_spend_seen": bool::default(),
///     "prunable_as_hex": String::default(),
///     "prunable_hash": String::default(),
///     "pruned_as_hex": String::default(),
///     "received_timestamp": u64::default(),
///     "relayed": bool::default(),
///     "tx_hash": String::default(),
///     "in_pool": bool::default(),
/// });
/// let tx_entry = from_value::<TxEntry>(json).unwrap();
/// assert!(matches!(tx_entry, TxEntry::NotInPool { .. }));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum TxEntry {
    /// This entry exists in the transaction pool.
    InPool {
        as_hex: String,
        /// `cuprate_rpc_types::json::tx::Transaction` should be used
        /// to create this JSON string in a type-safe manner.
        as_json: String,
        block_height: u64,
        block_timestamp: u64,
        confirmations: u64,
        double_spend_seen: bool,
        output_indices: Vec<u64>,
        prunable_as_hex: String,
        prunable_hash: String,
        pruned_as_hex: String,
        tx_hash: String,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_true"))]
        /// Will always be serialized as `true`.
        in_pool: bool,
    },
    /// This entry _does not_ exist in the transaction pool.
    NotInPool {
        as_hex: String,
        /// `cuprate_rpc_types::json::tx::Transaction` should be used
        /// to create this JSON string in a type-safe manner.
        as_json: String,
        double_spend_seen: bool,
        prunable_as_hex: String,
        prunable_hash: String,
        pruned_as_hex: String,
        received_timestamp: u64,
        relayed: bool,
        tx_hash: String,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_false"))]
        /// Will always be serialized as `false`.
        in_pool: bool,
    },
}

impl Default for TxEntry {
    fn default() -> Self {
        Self::NotInPool {
            as_hex: String::default(),
            as_json: String::default(),
            double_spend_seen: bool::default(),
            prunable_as_hex: String::default(),
            prunable_hash: String::default(),
            pruned_as_hex: String::default(),
            received_timestamp: u64::default(),
            relayed: bool::default(),
            tx_hash: String::default(),
            in_pool: false,
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
