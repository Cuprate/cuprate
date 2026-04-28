//! Output distributions for [`crate::json::GetOutputDistributionResponse`].

//---------------------------------------------------------------------------------------------------- Use
#[cfg(any(feature = "epee", feature = "serde"))]
use monero_oxide::io::VarInt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    container_as_blob::ContainerAsBlob,
    epee_object, error,
    macros::bytes::{Buf, BufMut},
    read_epee_value, read_marker, write_field, EpeeObject, EpeeObjectBuilder, EpeeValue,
};

//---------------------------------------------------------------------------------------------------- Free
/// Used for [`Distribution::CompressedBinary::distribution`].
#[doc = crate::macros::monero_definition_link!(
    "cc73fe71162d564ffda8e549b79a350bca53c454",
    "rpc/core_rpc_server_commands_defs.h",
    45..=55
)]
#[cfg(any(feature = "epee", feature = "serde"))]
fn compress_integer_array(v: &[u64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 2);
    for val in v {
        VarInt::write(val, &mut out).expect("Writing to vec should not fail");
    }
    out
}

/// Used for [`Distribution::CompressedBinary::distribution`].
#[doc = crate::macros::monero_definition_link!(
    "cc73fe71162d564ffda8e549b79a350bca53c454",
    "rpc/core_rpc_server_commands_defs.h",
    57..=72
)]
#[cfg(any(feature = "epee", feature = "serde"))]
fn decompress_integer_array(mut s: &[u8]) -> std::io::Result<Vec<u64>> {
    let mut v = Vec::new();
    while !s.is_empty() {
        v.push(VarInt::read(&mut s)?);
    }
    Ok(v)
}

//---------------------------------------------------------------------------------------------------- Distribution
#[doc = crate::macros::monero_definition_link!(
    "cc73fe71162d564ffda8e549b79a350bca53c454",
    "rpc/core_rpc_server_commands_defs.h",
    2468..=2508
)]
/// Used in [`crate::json::GetOutputDistributionResponse`].
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
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DistributionUncompressed {
    pub start_height: u64,
    pub base: u64,
    /// TODO: this is a binary JSON string if `binary == true`.
    pub distribution: Vec<u64>,
    pub amount: u64,
    pub binary: bool,
}

// Manual `Serialize` so the JSON output includes `compress: false`, matching
// monerod. `derive(Serialize)` would only emit the struct fields.
#[cfg(feature = "serde")]
impl Serialize for DistributionUncompressed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("DistributionUncompressed", 6)?;
        s.serialize_field("start_height", &self.start_height)?;
        s.serialize_field("base", &self.base)?;
        s.serialize_field("distribution", &self.distribution)?;
        s.serialize_field("amount", &self.amount)?;
        s.serialize_field("binary", &self.binary)?;
        s.serialize_field("compress", &false)?;
        s.end()
    }
}

// `DistributionUncompressed` is never used as a standalone EPEE object.
// Its `EpeeObject` impl exists only so that `Distribution::number_of_fields()`
// can call `s.number_of_fields()`. The write path is handled manually in
// `Distribution::write_fields` to correctly switch between blob and array
// encoding based on the `binary` flag — do NOT add a standalone write impl here.
#[cfg(feature = "epee")]
#[derive(Default)]
pub struct __DistributionUncompressedEpeeBuilder;

#[cfg(feature = "epee")]
impl EpeeObjectBuilder<DistributionUncompressed> for __DistributionUncompressedEpeeBuilder {
    fn add_field<B: Buf>(&mut self, _: &str, _: &mut B) -> error::Result<bool> {
        unreachable!("DistributionUncompressed is never deserialized as a standalone EPEE object")
    }

    fn finish(self) -> error::Result<DistributionUncompressed> {
        unreachable!("DistributionUncompressed is never deserialized as a standalone EPEE object")
    }
}

#[cfg(feature = "epee")]
impl EpeeObject for DistributionUncompressed {
    type Builder = __DistributionUncompressedEpeeBuilder;

    fn number_of_fields(&self) -> u64 {
        // start_height, base, amount, binary = 4
        // + distribution (skipped by write_field when empty)
        4 + u64::from(EpeeValue::should_write(&self.distribution))
    }

    fn write_fields<B: BufMut>(self, _: &mut B) -> error::Result<()> {
        unreachable!() // We don't write directly here, we do it in [`Distribution`]
    }
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
    Vec::<u8>::deserialize(d)
        .and_then(|v| decompress_integer_array(&v).map_err(serde::de::Error::custom))
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
        match name {
            "start_height" => self.start_height = Some(read_epee_value(r)?),
            "base" => self.base = Some(read_epee_value(r)?),
            "amount" => self.amount = Some(read_epee_value(r)?),
            "binary" => self.binary = Some(read_epee_value(r)?),
            "compress" => self.compress = Some(read_epee_value(r)?),
            "compressed_data" => self.compressed_data = Some(read_epee_value(r)?),
            // `distribution` arrives as a raw LE-u64 blob when `binary=true`
            // (monerod uses `KV_SERIALIZE_CONTAINER_POD_AS_BLOB_N`) or as a
            // typed EPEE u64 array when `binary=false`. Detect via marker.
            "distribution" => {
                let marker = read_marker(r)?;
                self.distribution = Some(if marker == ContainerAsBlob::<u64>::MARKER {
                    ContainerAsBlob::<u64>::read(r, &marker)?.into()
                } else {
                    Vec::<u64>::read(r, &marker)?
                });
            }
            _ => return Ok(false),
        }

        Ok(true)
    }

    fn finish(self) -> error::Result<Distribution> {
        const ELSE: error::Error = error::Error::Format("Required field was not found!");

        let start_height = self.start_height.ok_or(ELSE)?;
        let base = self.base.ok_or(ELSE)?;
        let amount = self.amount.ok_or(ELSE)?;

        let distribution = if let Some(compressed_data) = self.compressed_data {
            let distribution = decompress_integer_array(&compressed_data)
                .map_err(|_| error::Error::Format("Failed to decompress distribution"))?;
            Distribution::CompressedBinary(DistributionCompressedBinary {
                start_height,
                base,
                distribution,
                amount,
            })
        } else {
            let distribution = self.distribution.unwrap_or_default();
            Distribution::Uncompressed(DistributionUncompressed {
                binary: self.binary.unwrap_or_default(),
                distribution,
                start_height,
                base,
                amount,
            })
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
            Self::Uncompressed(DistributionUncompressed {
                start_height,
                base,
                distribution,
                amount,
                binary,
            }) => {
                write_field(start_height, "start_height", w)?;
                write_field(base, "base", w)?;
                if binary {
                    write_field(ContainerAsBlob::from(distribution), "distribution", w)?;
                } else {
                    write_field(distribution, "distribution", w)?;
                }
                write_field(amount, "amount", w)?;
                write_field(binary, "binary", w)?;
                write_field(false, "compress", w)?;
            }

            Self::CompressedBinary(DistributionCompressedBinary {
                start_height,
                base,
                distribution,
                amount,
            }) => {
                let compressed_data = compress_integer_array(&distribution);

                write_field(start_height, "start_height", w)?;
                write_field(base, "base", w)?;
                write_field(compressed_data, "compressed_data", w)?;
                write_field(amount, "amount", w)?;
                write_field(true, "binary", w)?;
                write_field(true, "compress", w)?;
            }
        }

        Ok(())
    }
}
