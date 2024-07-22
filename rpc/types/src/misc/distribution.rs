//! Output distributions for [`crate::json::GetOutputDistributionResponse`].

//---------------------------------------------------------------------------------------------------- Use
use std::mem::size_of;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    epee_object, error,
    macros::bytes::{Buf, BufMut},
    read_epee_value, read_varint, write_field, write_varint, EpeeObject, EpeeObjectBuilder,
    EpeeValue, Marker,
};

use super::OutputDistributionData;

//---------------------------------------------------------------------------------------------------- Free
/// Used for [`Distribution::CompressedBinary::compressed_data`].
///
/// TODO: for handler code. This should already be provided in the field.
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    45..=55
)]
#[allow(clippy::needless_pass_by_value)] // TODO: remove after impl
fn compress_integer_array(array: Vec<u64>) -> error::Result<Vec<u64>> {
    todo!()
}

/// Used for [`Distribution::CompressedBinary::compressed_data`].
///
/// TODO: for handler code. This should already be provided in the field.
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    57..=72
)]
#[allow(clippy::needless_pass_by_value)] // TODO: remove after impl
fn decompress_integer_array(array: Vec<u64>) -> Vec<u64> {
    todo!()
}

//---------------------------------------------------------------------------------------------------- Distribution
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
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
#[allow(dead_code, missing_docs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DistributionUncompressed {
    pub start_height: u64,
    pub base: u64,
    /// TODO: this is a binary JSON string if `binary == true`.
    pub distribution: Vec<u64>,
    pub amount: u64,
    pub binary: bool,
}

#[cfg(feature = "epee")]
epee_object! {
    DistributionUncompressed,
    start_height: u64,
    base: u64,
    distribution: Vec<u64>,
    amount: u64,
    binary: bool,
}

/// Data within [`Distribution::CompressedBinary`].
#[allow(dead_code, missing_docs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DistributionCompressedBinary {
    pub start_height: u64,
    pub base: u64,
    pub compressed_data: String,
    pub amount: u64,
}

#[cfg(feature = "epee")]
epee_object! {
    DistributionCompressedBinary,
    start_height: u64,
    base: u64,
    compressed_data: String,
    amount: u64,
}

//---------------------------------------------------------------------------------------------------- Epee
#[cfg(feature = "epee")]
/// [`EpeeObjectBuilder`] for [`Distribution`].
///
/// Not for public usage.
#[allow(dead_code, missing_docs)]
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct __DistributionEpeeBuilder {
    pub start_height: Option<u64>,
    pub base: Option<u64>,
    pub distribution: Option<Vec<u64>>,
    pub amount: Option<u64>,
    pub compressed_data: Option<String>,
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
            compress
        }

        self.compressed_data = read_epee_value(r).ok().unwrap_or_default();
        self.distribution = read_epee_value(r).ok().unwrap_or_default();

        Ok(true)
    }

    fn finish(self) -> error::Result<Distribution> {
        const ELSE: error::Error = error::Error::Format("Required field was not found!");

        let start_height = self.start_height.ok_or(ELSE)?;
        let base = self.base.ok_or(ELSE)?;
        let amount = self.amount.ok_or(ELSE)?;

        let distribution = if let Some(compressed_data) = self.compressed_data {
            Distribution::CompressedBinary(DistributionCompressedBinary {
                start_height,
                base,
                compressed_data,
                amount,
            })
        } else if let Some(distribution) = self.distribution {
            Distribution::Uncompressed(DistributionUncompressed {
                binary: self.binary.ok_or(ELSE)?,
                distribution,
                start_height,
                base,
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
        macro_rules! write_field {
            ($($field:ident),*) => {
                $(
                    write_field($field, stringify!($field), w)?;
                )*
            };
        }

        match self {
            Self::Uncompressed(s) => {
                s.write_fields(w)?;
                // This is on purpose `lower_case` instead of
                // `CONST_UPPER` due to `stringify!`.
                let compress = false;
                write_field!(compress);
            }

            Self::CompressedBinary(s) => {
                s.write_fields(w)?;
                let binary = true;
                let compress = true;
                write_field!(binary, compress);
            }
        }

        Ok(())
    }
}
