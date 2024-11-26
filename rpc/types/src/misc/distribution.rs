//! Output distributions for [`crate::json::GetOutputDistributionResponse`].

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    epee_object, error,
    macros::bytes::{Buf, BufMut},
    read_epee_value, write_field, EpeeObject, EpeeObjectBuilder, EpeeValue,
};

//---------------------------------------------------------------------------------------------------- Free
/// TODO: <https://github.com/Cuprate/cuprate/pull/229#discussion_r1690531904>.
///
/// Used for [`Distribution::CompressedBinary::distribution`].
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    45..=55
)]
#[cfg(any(feature = "epee", feature = "serde"))]
fn compress_integer_array(_: &[u64]) -> Vec<u8> {
    todo!()
}

/// TODO: <https://github.com/Cuprate/cuprate/pull/229#discussion_r1690531904>.
///
/// Used for [`Distribution::CompressedBinary::distribution`].
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    57..=72
)]
#[cfg(any(feature = "epee", feature = "serde"))]
fn decompress_integer_array(_: &[u8]) -> Vec<u64> {
    todo!()
}

//---------------------------------------------------------------------------------------------------- Distribution
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    2468..=2508
)]
/// Used in [`crate::json::GetOutputDistributionV2Response`].
///
/// # Internals
/// This type's (de)serialization depends on `monerod`'s (de)serialization.
///
/// During serialization:
/// [`Self::Uncompressed`] will emit:
/// - `compress: false`
///
/// [`Self::CompressedBinary`] will emit:
/// - `binary: true`
/// - `compress: true`
///
/// Upon deserialization, the presence of a `compressed_data`
/// field signifies that the [`Self::CompressedBinary`] should
/// be selected.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Distribution {
    /// Distribution data will be (de)serialized as either JSON or binary (uncompressed).
    Uncompressed(DistributionUncompressed),
    /// Distribution data will be (de)serialized as compressed binary.
    CompressedBinary(DistributionCompressedBinary),
}

impl Default for Distribution {
    fn default() -> Self {
        Self::Uncompressed(DistributionUncompressed::default())
    }
}

/// Data within [`Distribution::Uncompressed`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DistributionUncompressed {
    pub start_height: u64,
    pub base: u64,
    pub distribution: Vec<u64>,
    pub amount: u64,
}

#[cfg(feature = "epee")]
epee_object! {
    DistributionUncompressed,
    start_height: u64,
    base: u64,
    distribution: Vec<u64>,
    amount: u64,
}

/// Data within [`Distribution::CompressedBinary`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DistributionCompressedBinary {
    pub start_height: u64,
    pub base: u64,
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "serialize_distribution_as_compressed_data")
    )]
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "deserialize_compressed_data_as_distribution")
    )]
    #[cfg_attr(feature = "serde", serde(rename = "compressed_data"))]
    pub distribution: Vec<u64>,
    pub amount: u64,
}

#[cfg(feature = "epee")]
epee_object! {
    DistributionCompressedBinary,
    start_height: u64,
    base: u64,
    distribution: Vec<u64>,
    amount: u64,
}

/// Serializer function for [`DistributionCompressedBinary::distribution`].
///
/// 1. Compresses the distribution array
/// 2. Serializes the compressed data
#[cfg(feature = "serde")]
#[expect(clippy::ptr_arg)]
fn serialize_distribution_as_compressed_data<S>(v: &Vec<u64>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    compress_integer_array(v).serialize(s)
}

/// Deserializer function for [`DistributionCompressedBinary::distribution`].
///
/// 1. Deserializes as `compressed_data` field.
/// 2. Decompresses and returns the data
#[cfg(feature = "serde")]
fn deserialize_compressed_data_as_distribution<'de, D>(d: D) -> Result<Vec<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Vec::<u8>::deserialize(d).map(|v| decompress_integer_array(&v))
}

//---------------------------------------------------------------------------------------------------- Epee
#[cfg(feature = "epee")]
/// [`EpeeObjectBuilder`] for [`Distribution`].
///
/// Not for public usage.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct __DistributionEpeeBuilder {
    pub start_height: Option<u64>,
    pub base: Option<u64>,
    pub distribution: Option<Vec<u64>>,
    pub amount: Option<u64>,
    pub compressed_data: Option<Vec<u8>>,
    pub binary: Option<bool>,
    pub compress: Option<bool>,
}

#[cfg(feature = "epee")]
impl EpeeObjectBuilder<Distribution> for __DistributionEpeeBuilder {
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
            start_height,
            base,
            amount,
            binary,
            compress,
            compressed_data,
            distribution
        }

        Ok(true)
    }

    fn finish(self) -> error::Result<Distribution> {
        const ELSE: error::Error = error::Error::Format("Required field was not found!");

        let start_height = self.start_height.ok_or(ELSE)?;
        let base = self.base.ok_or(ELSE)?;
        let amount = self.amount.ok_or(ELSE)?;

        let distribution = if let Some(compressed_data) = self.compressed_data {
            let distribution = decompress_integer_array(&compressed_data);
            Distribution::CompressedBinary(DistributionCompressedBinary {
                start_height,
                base,
                distribution,
                amount,
            })
        } else if let Some(distribution) = self.distribution {
            Distribution::Uncompressed(DistributionUncompressed {
                start_height,
                base,
                distribution,
                amount,
            })
        } else {
            return Err(ELSE);
        };

        Ok(distribution)
    }
}

#[cfg(feature = "epee")]
impl EpeeObject for Distribution {
    type Builder = __DistributionEpeeBuilder;

    fn number_of_fields(&self) -> u64 {
        match self {
            // Inner struct fields + `compress`.
            Self::Uncompressed(s) => s.number_of_fields() + 1,
            // Inner struct fields + `compress` + `binary`.
            Self::CompressedBinary(s) => s.number_of_fields() + 2,
        }
    }

    fn write_fields<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        match self {
            Self::Uncompressed(s) => {
                s.write_fields(w)?;
                write_field(false, "compress", w)?;
            }

            Self::CompressedBinary(DistributionCompressedBinary {
                start_height,
                base,
                distribution,
                amount,
            }) => {
                let compressed_data = compress_integer_array(&distribution);

                start_height.write(w)?;
                base.write(w)?;
                compressed_data.write(w)?;
                amount.write(w)?;

                write_field(true, "binary", w)?;
                write_field(true, "compress", w)?;
            }
        }

        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod tests {
    // use pretty_assertions::assert_eq;

    // use super::*;

    // TODO: re-enable tests after (de)compression functions are implemented.

    // /// Tests that [`compress_integer_array`] outputs as expected.
    // #[test]
    // fn compress() {
    //     let varints = &[16_384, 16_383, 16_382, 16_381];
    //     let bytes = compress_integer_array(varints).unwrap();

    //     let expected = [2, 0, 1, 0, 253, 255, 249, 255, 245, 255];
    //     assert_eq!(expected, *bytes);
    // }

    // /// Tests that [`decompress_integer_array`] outputs as expected.
    // #[test]
    // fn decompress() {
    //     let bytes = &[2, 0, 1, 0, 253, 255, 249, 255, 245, 255];
    //     let varints = decompress_integer_array(bytes);

    //     let expected = vec![16_384, 16_383, 16_382, 16_381];
    //     assert_eq!(expected, varints);
    // }
}
