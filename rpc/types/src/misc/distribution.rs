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

use super::OutputDistributionData;

//---------------------------------------------------------------------------------------------------- Free
/// TODO
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    45..=55
)]
pub fn compress_integer_array(array: Vec<u64>) -> Vec<u64> {
    todo!();
}

/// TODO
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    57..=72
)]
pub fn decompress_integer_array(array: Vec<u64>) -> Vec<u64> {
    todo!();
}

//---------------------------------------------------------------------------------------------------- Distribution
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    2468..=2508
)]
/// Used in [`crate::json::GetOutputDistributionResponse`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Distribution {
    /// TODO
    Json {
        data: OutputDistributionData,
        amount: u64,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_false"))]
        /// Will always be serialized as `false`.
        binary: bool,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_false"))]
        /// Will always be serialized as `false`.
        compress: bool,
    },
    /// TODO
    Binary {
        data: OutputDistributionData,
        amount: u64,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_true"))]
        /// Will always be serialized as `true`.
        binary: bool,
        #[cfg_attr(feature = "serde", serde(serialize_with = "serde_false"))]
        /// Will always be serialized as `false`.
        compress: bool,
    },
    /// TODO
    CompressedBinary {
        data: OutputDistributionData,
        amount: u64,
        compressed_data: String,
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
        Self::Json {
            data: OutputDistributionData::default(),
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
    pub data: Option<OutputDistributionData>,
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
            data,
            amount,
            compressed_data,
            binary,
            compress
        }

        Ok(true)
    }

    fn finish(self) -> error::Result<Distribution> {
        const ELSE: error::Error = error::Error::Format("Required field was not found!");

        let data = self.data.ok_or(ELSE)?;
        let amount = self.amount.ok_or(ELSE)?;
        let binary = self.binary.ok_or(ELSE)?;
        let compress = self.compress.ok_or(ELSE)?;

        let distribution = if binary && compress {
            Distribution::CompressedBinary {
                compressed_data: self.compressed_data.ok_or(ELSE)?,
                data,
                amount,
                binary,
                compress,
            }
        } else if binary {
            Distribution::Binary {
                data,
                amount,
                binary,
                compress,
            }
        } else {
            Distribution::Json {
                data,
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
        match self {
            Self::Json { .. } => 4,
            Self::Binary { .. } => 4,
            Self::CompressedBinary { .. } => 5,
        }
    }

    fn write_fields<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        match self {
            Self::Json {
                data,
                amount,
                binary,
                compress,
            }
            | Self::Binary {
                data,
                amount,
                binary,
                compress,
            } => {
                if amount.should_write() {
                    write_field(amount, "amount", w)?;
                }
                if binary.should_write() {
                    write_field(binary, "binary", w)?;
                }
                if compress.should_write() {
                    write_field(compress, "compress", w)?;
                }
                data.write(w)?;
            }

            Self::CompressedBinary {
                data,
                amount,
                compressed_data,
                binary,
                compress,
            } => {
                if amount.should_write() {
                    write_field(amount, "amount", w)?;
                }
                if binary.should_write() {
                    write_field(binary, "binary", w)?;
                }
                if compress.should_write() {
                    write_field(compress, "compress", w)?;
                }
                if data.start_height.should_write() {
                    write_field(data.start_height, "start_height", w)?;
                }
                if data.base.should_write() {
                    write_field(data.base, "base", w)?;
                }

                // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L2487>
                // TODO: cast `String` -> `Vec<u64>`
                let compressed_data = compress_integer_array(compressed_data);
                if compressed_data.should_write() {
                    compressed_data.write(w)?;
                }
            }
        }

        Ok(())
    }
}
