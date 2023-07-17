use epee_encoding::{
    error::Error,
    io::{Read, Write},
    marker::InnerMarker,
    write_field, EpeeObject, EpeeObjectBuilder, EpeeValue,
};

use super::{PrunedTxBlobEntry, TransactionBlobs};

impl EpeeObject for TransactionBlobs {
    type Builder = TransactionBlobsBuilder;
    fn number_of_fields(&self) -> u64 {
        1
    }
    fn write_fields<W: Write>(&self, w: &mut W) -> epee_encoding::error::Result<()> {
        match self {
            TransactionBlobs::Pruned(txs) => write_field(txs, "txs", w),
            TransactionBlobs::Normal(txs) => write_field(txs, "txs", w),
        }
    }
}

#[derive(Default)]
pub enum TransactionBlobsBuilder {
    #[default]
    Init,
    Pruned(Vec<PrunedTxBlobEntry>),
    Normal(Vec<Vec<u8>>),
}

impl EpeeObjectBuilder<TransactionBlobs> for TransactionBlobsBuilder {
    fn add_field<R: Read>(&mut self, name: &str, r: &mut R) -> epee_encoding::error::Result<bool> {
        if name != "txs" {
            return Ok(false);
        }

        let marker = epee_encoding::read_marker(r)?;

        if !marker.is_seq {
            return Err(Error::Format("Expected a sequence but got a single value"));
        }

        match marker.inner_marker {
            InnerMarker::String => {
                let state = TransactionBlobsBuilder::Normal(Vec::<Vec<u8>>::read(r, &marker)?);
                let _ = std::mem::replace(self, state);
            }
            InnerMarker::Object => {
                let state =
                    TransactionBlobsBuilder::Pruned(Vec::<PrunedTxBlobEntry>::read(r, &marker)?);
                let _ = std::mem::replace(self, state);
            }

            _ => return Err(Error::Format("Unexpected marker")),
        }

        Ok(true)
    }

    fn finish(self) -> epee_encoding::error::Result<TransactionBlobs> {
        match self {
            TransactionBlobsBuilder::Init => Err(Error::Format("Required field was not in data")),
            TransactionBlobsBuilder::Normal(txs) => Ok(TransactionBlobs::Normal(txs)),
            TransactionBlobsBuilder::Pruned(txs) => Ok(TransactionBlobs::Pruned(txs)),
        }
    }
}
