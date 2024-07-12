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

use crate::misc::TxBlobEntry;

//---------------------------------------------------------------------------------------------------- BlockCompleteEntry
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    210..=221
)]
/// Used in [`crate::bin::GetBlocksResponse`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BlockCompleteEntry {
    pub pruned: bool,
    pub block: String,
    pub block_weight: u64,
    pub txs: Vec<TxBlobEntry>,
}

//---------------------------------------------------------------------------------------------------- Serde
#[cfg(feature = "epee")]
/// [`EpeeObjectBuilder`] for [`BlockCompleteEntry`].
///
/// Not for public usage.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct __BlockCompleteEntryEpeeBuilder {
    pub pruned: Option<bool>,
    pub block: Option<String>,
    pub block_weight: Option<u64>,
    pub txs: Option<Vec<TxBlobEntry>>,
}

#[cfg(feature = "epee")]
impl EpeeObjectBuilder<BlockCompleteEntry> for __BlockCompleteEntryEpeeBuilder {
    fn add_field<B: Buf>(&mut self, name: &str, r: &mut B) -> error::Result<bool> {
        match name {
            "pruned" => self.pruned = Some(read_epee_value(r)?),
            "block" => self.block = Some(read_epee_value(r)?),
            "block_weight" => self.block_weight = Some(read_epee_value(r)?),
            "txs" => self.txs = Some(read_epee_value(r)?),
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn finish(self) -> error::Result<BlockCompleteEntry> {
        const ELSE: error::Error = error::Error::Format("Required field was not found!");
        Ok(BlockCompleteEntry {
            pruned: self.pruned.unwrap_or(false),
            block: self.block.ok_or(ELSE)?,
            block_weight: self.block_weight.unwrap_or(0),
            txs: self.txs.ok_or(ELSE)?,
        })
    }
}

#[cfg(feature = "epee")]
impl EpeeObject for BlockCompleteEntry {
    type Builder = __BlockCompleteEntryEpeeBuilder;

    fn number_of_fields(&self) -> u64 {
        4
    }

    fn write_fields<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        write_field(self.pruned, "pruned", w)?;
        write_field(self.block, "block", w)?;
        write_field(self.block_weight, "block_weight", w)?;

        // The following section is why custom epee (de)serialization exists.
        //
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L142-L162>

        if self.pruned {
            write_field(self.txs, "txs", w)?;
            return Ok(());
        }

        let txs: Vec<String> = if self.txs.should_write() {
            self.txs.into_iter().map(|i| i.blob).collect()
        } else {
            Vec::new()
        };

        write_field(txs, "txs", w)?;

        // TODO: what is the purpose of these line?
        // We take `self` so it gets destructed after this function,
        // is there a need to do this swap?
        //
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L155-L161>

        Ok(())
    }
}
