//! Output distributions for [`crate::json::GetOutputDistributionResponse`].

//---------------------------------------------------------------------------------------------------- Use
use std::mem::size_of;

#[cfg(feature = "serde")]
use crate::serde::{serde_false, serde_true};
use cuprate_epee_encoding::read_varint;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    epee_object, error,
    macros::bytes::{Buf, BufMut},
    read_epee_value, write_field, write_varint, EpeeObject, EpeeObjectBuilder, EpeeValue, Marker,
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
/// This enum's variant depends upon the `binary` and `compress` fields.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Distribution {
    /// Distribution data will be (de)serialized as either JSON or binary (uncompressed).
    Uncompressed {
        start_height: u64,
        base: u64,
        /// TODO: this is a binary JSON string if `binary == true`.
        ///
        /// Considering both the `binary` field and `/get_output_distribution.bin`
        /// endpoint are undocumented in the first place, Cuprate could just drop support for this.
        distribution: Vec<u64>,
        amount: u64,
        binary: bool,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_false"))]
        /// Will always be serialized as `false`.
        compress: bool,
    },
    /// Distribution data will be (de)serialized as compressed binary.
    CompressedBinary {
        start_height: u64,
        base: u64,
        compressed_data: String,
        amount: u64,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_true"))]
        /// Will always be serialized as `true`.
        binary: bool,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_true"))]
        /// Will always be serialized as `true`.
        compress: bool,
    },
}

impl Default for Distribution {
    fn default() -> Self {
        Self::Uncompressed {
            start_height: u64::default(),
            base: u64::default(),
            distribution: Vec::<u64>::default(),
            amount: u64::default(),
            binary: false,
            compress: false,
        }
    }
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
            distribution,
            amount,
            compressed_data,
            binary,
            compress
        }

        Ok(true)
    }

    fn finish(self) -> error::Result<Distribution> {
        const ELSE: error::Error = error::Error::Format("Required field was not found!");

        let start_height = self.start_height.ok_or(ELSE)?;
        let base = self.base.ok_or(ELSE)?;
        let amount = self.amount.ok_or(ELSE)?;
        let binary = self.binary.ok_or(ELSE)?;
        let compress = self.compress.ok_or(ELSE)?;

        let distribution = if binary && compress {
            Distribution::CompressedBinary {
                compressed_data: self.compressed_data.ok_or(ELSE)?,
                start_height,
                base,
                amount,
                binary,
                compress,
            }
        } else {
            Distribution::Uncompressed {
                distribution: self.distribution.ok_or(ELSE)?,
                start_height,
                base,
                amount,
                binary,
                compress,
            }
        };

        Ok(distribution)
    }
}

#[cfg(feature = "epee")]
impl EpeeObject for Distribution {
    type Builder = __DistributionEpeeBuilder;

    fn number_of_fields(&self) -> u64 {
        let mut fields = 0;

        macro_rules! add_field {
            ($($field:ident),*) => {
                $(
                    if $field.should_write() {
                        fields += 1;
                    }
                )*
            };
        }

        match self {
            Self::Uncompressed {
                distribution,
                start_height,
                base,
                amount,
                binary,
                compress,
            } => {
                const COMPRESS: bool = false;
                add_field! {
                    distribution,
                    start_height,
                    base,
                    amount,
                    binary,
                    COMPRESS
                }
            }
            Self::CompressedBinary {
                start_height,
                base,
                compressed_data,
                amount,
                binary,
                compress,
            } => {
                const BINARY: bool = true;
                const COMPRESS: bool = true;
                add_field! {
                    start_height,
                    base,
                    compressed_data,
                    amount,
                    BINARY,
                    COMPRESS
                }
            }
        }

        fields
    }

    fn write_fields<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        macro_rules! write_field {
            ($($field:ident),*) => {
                $(
                    if $field.should_write() {
                        write_field($field, stringify!($field), w)?;
                    }
                )*
            };
        }

        match self {
            Self::Uncompressed {
                distribution,
                start_height,
                base,
                amount,
                binary,
                compress,
            } => {
                // This is on purpose `lower_case` instead of
                // `CONST_UPPER` due to `stringify!`.
                let compress = false;
                write_field! {
                    distribution,
                    start_height,
                    base,
                    amount,
                    binary,
                    compress
                }
            }

            Self::CompressedBinary {
                start_height,
                base,
                compressed_data,
                amount,
                binary,
                compress,
            } => {
                let binary = true;
                let compress = true;
                write_field! {
                    start_height,
                    base,
                    compressed_data,
                    amount,
                    binary,
                    compress
                }
            }
        }

        Ok(())
    }
}
