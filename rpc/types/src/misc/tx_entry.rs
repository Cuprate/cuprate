//! TODO

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use crate::serde::{serde_false, serde_true};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    epee_object, error,
    macros::bytes::{Buf, BufMut},
    read_epee_value, write_field, EpeeObject, EpeeObjectBuilder, EpeeValue, Marker,
};

//---------------------------------------------------------------------------------------------------- TxEntry
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    389..=428
)]
/// Used in [`crate::other::GetTransactionsResponse`].
///
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

//---------------------------------------------------------------------------------------------------- Serde
#[cfg(feature = "epee")]
/// [`EpeeObjectBuilder`] for [`TxEntry`].
///
/// Not for public usage.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct __TxEntryEpeeBuilder {
    pub as_hex: Option<String>,
    pub as_json: Option<String>,
    pub block_height: Option<u64>,
    pub block_timestamp: Option<u64>,
    pub confirmations: Option<u64>,
    pub double_spend_seen: Option<bool>,
    pub in_pool: Option<bool>,
    pub output_indices: Option<Vec<u64>>,
    pub prunable_as_hex: Option<String>,
    pub prunable_hash: Option<String>,
    pub pruned_as_hex: Option<String>,
    pub received_timestamp: Option<u64>,
    pub relayed: Option<bool>,
    pub tx_hash: Option<String>,
}

#[cfg(feature = "epee")]
impl EpeeObjectBuilder<TxEntry> for __TxEntryEpeeBuilder {
    fn add_field<B: Buf>(&mut self, name: &str, r: &mut B) -> error::Result<bool> {
        macro_rules! read_epee_field {
            ($($field:ident),*) => {
                match name {
                    $(
                        stringify!($field) => { self.$field = Some(read_epee_value(r)?); },
                    )*
                    _ => return Ok(false),
                }
            };
        }

        read_epee_field! {
            as_hex,
            as_json,
            block_height,
            block_timestamp,
            confirmations,
            double_spend_seen,
            in_pool,
            output_indices,
            prunable_as_hex,
            prunable_hash,
            pruned_as_hex,
            received_timestamp,
            relayed,
            tx_hash
        }

        Ok(true)
    }

    fn finish(self) -> error::Result<TxEntry> {
        const ELSE: error::Error = error::Error::Format("Required field was not found!");

        let in_pool = self.in_pool.ok_or(ELSE)?;

        let tx_entry = if in_pool {
            TxEntry::InPool {
                as_hex: self.as_hex.ok_or(ELSE)?,
                as_json: self.as_json.ok_or(ELSE)?,
                block_height: self.block_height.ok_or(ELSE)?,
                block_timestamp: self.block_timestamp.ok_or(ELSE)?,
                confirmations: self.confirmations.ok_or(ELSE)?,
                double_spend_seen: self.double_spend_seen.ok_or(ELSE)?,
                in_pool: self.in_pool.ok_or(ELSE)?,
                output_indices: self.output_indices.ok_or(ELSE)?,
                prunable_as_hex: self.prunable_as_hex.ok_or(ELSE)?,
                prunable_hash: self.prunable_hash.ok_or(ELSE)?,
                pruned_as_hex: self.pruned_as_hex.ok_or(ELSE)?,
                tx_hash: self.tx_hash.ok_or(ELSE)?,
            }
        } else {
            TxEntry::NotInPool {
                as_hex: self.as_hex.ok_or(ELSE)?,
                as_json: self.as_json.ok_or(ELSE)?,
                double_spend_seen: self.double_spend_seen.ok_or(ELSE)?,
                in_pool: self.in_pool.ok_or(ELSE)?,
                prunable_as_hex: self.prunable_as_hex.ok_or(ELSE)?,
                prunable_hash: self.prunable_hash.ok_or(ELSE)?,
                pruned_as_hex: self.pruned_as_hex.ok_or(ELSE)?,
                received_timestamp: self.received_timestamp.ok_or(ELSE)?,
                relayed: self.relayed.ok_or(ELSE)?,
                tx_hash: self.tx_hash.ok_or(ELSE)?,
            }
        };

        Ok(tx_entry)
    }
}

#[cfg(feature = "epee")]
impl EpeeObject for TxEntry {
    type Builder = __TxEntryEpeeBuilder;

    fn number_of_fields(&self) -> u64 {
        match self {
            Self::InPool { .. } => 12,
            Self::NotInPool { .. } => 10,
        }
    }

    fn write_fields<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        macro_rules! write_fields {
            ($($field:ident),*) => {
                $(
                    write_field($field, stringify!($field), w)?;
                )*
            };
        }

        match self {
            Self::InPool {
                as_hex,
                as_json,
                block_height,
                block_timestamp,
                confirmations,
                double_spend_seen,
                output_indices,
                prunable_as_hex,
                prunable_hash,
                pruned_as_hex,
                tx_hash,
                in_pool,
            } => {
                write_fields! {
                    as_hex,
                    as_json,
                    block_height,
                    block_timestamp,
                    confirmations,
                    double_spend_seen,
                    output_indices,
                    prunable_as_hex,
                    prunable_hash,
                    pruned_as_hex,
                    tx_hash,
                    in_pool
                }
            }
            Self::NotInPool {
                as_hex,
                as_json,
                double_spend_seen,
                prunable_as_hex,
                prunable_hash,
                pruned_as_hex,
                received_timestamp,
                relayed,
                tx_hash,
                in_pool,
            } => {
                write_fields! {
                    as_hex,
                    as_json,
                    double_spend_seen,
                    prunable_as_hex,
                    prunable_hash,
                    pruned_as_hex,
                    received_timestamp,
                    relayed,
                    tx_hash,
                    in_pool
                }
            }
        }

        Ok(())
    }
}
