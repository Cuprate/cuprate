use serde::de::{Error, SeqAccess};
use serde::ser::SerializeSeq;
use serde::{
    de::{Deserialize, Visitor},
    Deserializer, Serialize, Serializer,
};
use std::fmt::Formatter;

use super::TransactionBlobs;

impl<'de> Deserialize<'de> for TransactionBlobs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TBVisitor;

        impl<'de> Visitor<'de> for TBVisitor {
            type Value = TransactionBlobs;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                write!(formatter, "A sequence of transactions blob")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut normal = Vec::new();
                //let pruned = Vec::new();

                while let Some(val) = seq.next_element::<SingleBlob>()? {
                    match val {
                        SingleBlob::Pruned(tx) => normal.push(tx),
                    }
                }

                Ok(TransactionBlobs::Normal(normal))
            }
        }

        deserializer.deserialize_any(TBVisitor)
    }
}

impl Serialize for TransactionBlobs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TransactionBlobs::Pruned(_) => todo!(),
            TransactionBlobs::Normal(txs) => {
                let mut seq_ser = serializer.serialize_seq(Some(txs.len()))?;
                for tx in txs {
                    seq_ser.serialize_element(tx)?;
                }
                seq_ser.end()
            }
        }
    }
}

enum SingleBlob {
    Pruned(Vec<u8>),
}

impl<'de> Deserialize<'de> for SingleBlob {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TBDSVisitor;

        impl<'de> Visitor<'de> for TBDSVisitor {
            type Value = SingleBlob;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                write!(formatter, "A single transaction blob")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(SingleBlob::Pruned(v.into()))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(SingleBlob::Pruned(v))
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                todo!("Pruned blobs")
            }
        }

        deserializer.deserialize_any(TBDSVisitor)
    }
}
