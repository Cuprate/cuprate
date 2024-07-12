//! TODO

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    epee_object, error,
    macros::bytes::{Buf, BufMut},
    read_epee_value, write_field, EpeeObject, EpeeObjectBuilder, EpeeValue,
};

//---------------------------------------------------------------------------------------------------- TxEntry
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    389..=428
)]
/// Used in [`crate::other::GetTransactionsResponse`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TxEntry {
    pub as_hex: String,
    pub as_json: String,
    pub block_height: u64,
    pub block_timestamp: u64,
    pub confirmations: u64,
    pub double_spend_seen: bool,
    pub in_pool: bool,
    pub output_indices: Vec<u64>,
    pub prunable_as_hex: String,
    pub prunable_hash: String,
    pub pruned_as_hex: String,
    pub received_timestamp: u64,
    pub relayed: bool,
    pub tx_hash: String,
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
        Ok(TxEntry {
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
            received_timestamp: self.received_timestamp.ok_or(ELSE)?,
            relayed: self.relayed.ok_or(ELSE)?,
            tx_hash: self.tx_hash.ok_or(ELSE)?,
        })
    }
}

#[cfg(feature = "epee")]
impl EpeeObject for TxEntry {
    type Builder = __TxEntryEpeeBuilder;

    fn number_of_fields(&self) -> u64 {
        // TODO: this is either 12 or 10 depending on `self.in_pool`.
        14
    }

    fn write_fields<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        write_field(self.as_hex, "as_hex", w)?;
        write_field(self.as_json, "as_json", w)?;
        write_field(self.double_spend_seen, "double_spend_seen", w)?;
        write_field(self.in_pool, "in_pool", w)?;
        write_field(self.prunable_as_hex, "prunable_as_hex", w)?;
        write_field(self.prunable_hash, "prunable_hash", w)?;
        write_field(self.pruned_as_hex, "pruned_as_hex", w)?;
        write_field(self.tx_hash, "tx_hash", w)?;

        // The following section is why custom epee (de)serialization exists.
        //
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L406-L427>

        if self.in_pool {
            write_field(self.block_height, "block_height", w)?;
            write_field(self.confirmations, "confirmations", w)?;
            write_field(self.block_timestamp, "block_timestamp", w)?;
            write_field(self.output_indices, "output_indices", w)?;
        } else {
            write_field(self.relayed, "relayed", w)?;
            write_field(self.received_timestamp, "received_timestamp", w)?;
        }

        Ok(())
    }
}
