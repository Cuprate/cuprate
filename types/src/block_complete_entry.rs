//! TODO

//---------------------------------------------------------------------------------------------------- Import
#[cfg(feature = "epee")]
use bytes::Bytes;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_fixed_bytes::ByteArray;

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    epee_object,
    macros::bytes::{Buf, BufMut},
    EpeeValue, InnerMarker,
};

//---------------------------------------------------------------------------------------------------- BlockCompleteEntry
/// A block that can contain transactions.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BlockCompleteEntry {
    /// `true` if transaction data is pruned.
    pub pruned: bool,
    /// The block.
    pub block: Bytes,
    /// The block weight/size.
    pub block_weight: u64,
    /// The block's transactions.
    pub txs: TransactionBlobs,
}

#[cfg(feature = "epee")]
epee_object!(
    BlockCompleteEntry,
    pruned: bool = false,
    block: Bytes,
    block_weight: u64 = 0_u64,
    txs: TransactionBlobs = TransactionBlobs::None =>
        TransactionBlobs::tx_blob_read,
        TransactionBlobs::tx_blob_write,
        TransactionBlobs::should_write_tx_blobs,
);

//---------------------------------------------------------------------------------------------------- TransactionBlobs
/// TODO
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TransactionBlobs {
    /// TODO
    Pruned(Vec<PrunedTxBlobEntry>),
    /// TODO
    Normal(Vec<Bytes>),
    #[default]
    /// TODO
    None,
}

impl TransactionBlobs {
    /// TODO
    pub fn take_pruned(self) -> Option<Vec<PrunedTxBlobEntry>> {
        match self {
            Self::Normal(_) => None,
            Self::Pruned(txs) => Some(txs),
            Self::None => Some(vec![]),
        }
    }

    /// TODO
    pub fn take_normal(self) -> Option<Vec<Bytes>> {
        match self {
            Self::Normal(txs) => Some(txs),
            Self::Pruned(_) => None,
            Self::None => Some(vec![]),
        }
    }

    /// TODO
    pub fn len(&self) -> usize {
        match self {
            Self::Normal(txs) => txs.len(),
            Self::Pruned(txs) => txs.len(),
            Self::None => 0,
        }
    }

    /// TODO
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// TODO
    #[cfg(feature = "epee")]
    fn tx_blob_read<B: Buf>(b: &mut B) -> cuprate_epee_encoding::Result<Self> {
        let marker = cuprate_epee_encoding::read_marker(b)?;
        match marker.inner_marker {
            InnerMarker::Object => Ok(Self::Pruned(Vec::read(b, &marker)?)),
            InnerMarker::String => Ok(Self::Normal(Vec::read(b, &marker)?)),
            _ => Err(cuprate_epee_encoding::Error::Value(
                "Invalid marker for tx blobs".to_string(),
            )),
        }
    }

    /// TODO
    #[cfg(feature = "epee")]
    fn tx_blob_write<B: BufMut>(
        self,
        field_name: &str,
        w: &mut B,
    ) -> cuprate_epee_encoding::Result<()> {
        if self.should_write_tx_blobs() {
            match self {
                Self::Normal(bytes) => {
                    cuprate_epee_encoding::write_field(bytes, field_name, w)?;
                }
                Self::Pruned(obj) => {
                    cuprate_epee_encoding::write_field(obj, field_name, w)?;
                }
                Self::None => (),
            }
        }
        Ok(())
    }

    /// TODO
    #[cfg(feature = "epee")]
    fn should_write_tx_blobs(&self) -> bool {
        !self.is_empty()
    }
}

//---------------------------------------------------------------------------------------------------- PrunedTxBlobEntry
/// A pruned transaction with the hash of the missing prunable data
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PrunedTxBlobEntry {
    /// The transaction.
    pub tx: Bytes,
    /// The prunable transaction hash.
    pub prunable_hash: ByteArray<32>,
}

#[cfg(feature = "epee")]
epee_object!(
    PrunedTxBlobEntry,
    tx: Bytes,
    prunable_hash: ByteArray<32>,
);

//---------------------------------------------------------------------------------------------------- Import
#[cfg(test)]
mod tests {}
