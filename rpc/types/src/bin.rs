//! Binary types from [`.bin` endpoints](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin).
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
use cuprate_fixed_bytes::ByteArrayVec;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    container_as_blob::ContainerAsBlob,
    epee_object, error,
    macros::bytes::{Buf, BufMut},
    read_epee_value, write_field, EpeeObject, EpeeObjectBuilder,
};

use cuprate_types::BlockCompleteEntry;

use crate::{
    base::AccessResponseBase,
    macros::{define_request, define_request_and_response, define_request_and_response_doc},
    misc::{BlockOutputIndices, GetOutputsOut, OutKeyBin, PoolTxInfo, Status},
    rpc_call::RpcCallValue,
};

#[cfg(any(feature = "epee", feature = "serde"))]
use crate::defaults::{default_false, default_zero};
#[cfg(feature = "epee")]
use crate::misc::PoolInfoExtent;

//---------------------------------------------------------------------------------------------------- Definitions
define_request_and_response! {
    get_blocks_by_heightbin,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 264..=286,
    GetBlocksByHeight,
    Request {
        heights: Vec<u64>,
    },
    AccessResponseBase {
        blocks: Vec<BlockCompleteEntry>,
    }
}

define_request_and_response! {
    get_hashesbin,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 309..=338,
    GetHashes,
    Request {
        block_ids: ByteArrayVec<32>,
        start_height: u64,
    },
    AccessResponseBase {
        m_blocks_ids: ByteArrayVec<32>,
        start_height: u64,
        current_height: u64,
    }
}

#[cfg(not(feature = "epee"))]
define_request_and_response! {
    get_o_indexesbin,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 487..=510,
    GetOutputIndexes,
    #[derive(Copy)]
    Request {
        txid: [u8; 32],
    },
    AccessResponseBase {
        o_indexes: Vec<u64>,
    }
}

#[cfg(feature = "epee")]
define_request_and_response! {
    get_o_indexesbin,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 487..=510,
    GetOutputIndexes,
    #[derive(Copy)]
    Request {
        txid: [u8; 32],
    },
    AccessResponseBase {
        o_indexes: Vec<u64> as ContainerAsBlob<u64>,
    }
}

define_request_and_response! {
    get_outsbin,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 512..=565,
    GetOuts,
    Request {
        outputs: Vec<GetOutputsOut>,
        get_txid: bool = default_false(), "default_false",
    },
    AccessResponseBase {
        outs: Vec<OutKeyBin>,
    }
}

define_request_and_response! {
    get_transaction_pool_hashesbin,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1593..=1613,
    GetTransactionPoolHashes,
    Request {},
    AccessResponseBase {
        tx_hashes: ByteArrayVec<32>,
    }
}

//---------------------------------------------------------------------------------------------------- GetBlocks
define_request! {
    #[doc = define_request_and_response_doc!(
        "response" => GetBlocksResponse,
        get_blocksbin,
        cc73fe71162d564ffda8e549b79a350bca53c454,
        core_rpc_server_commands_defs, h, 162, 262,
    )]
    GetBlocksRequest {
        requested_info: u8 = default_zero::<u8>(), "default_zero",
        // FIXME: This is a `std::list` in `monerod` because...?
        block_ids: ByteArrayVec<32>,
        start_height: u64,
        prune: bool,
        no_miner_tx: bool = default_false(), "default_false",
        pool_info_since: u64 = default_zero::<u64>(), "default_zero",
    }
}

// TODO: add `top_block_hash` field
// <https://github.com/monero-project/monero/blame/893916ad091a92e765ce3241b94e706ad012b62a/src/rpc/core_rpc_server_commands_defs.h#L263>
#[doc = define_request_and_response_doc!(
    "request" => GetBlocksRequest,
    get_blocksbin,
    cc73fe71162d564ffda8e549b79a350bca53c454,
    core_rpc_server_commands_defs, h, 162, 262,
)]
///
/// This response's variant depends upon [`PoolInfoExtent`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GetBlocksResponse {
    /// Will always serialize a [`PoolInfoExtent::None`] field.
    PoolInfoNone(GetBlocksResponsePoolInfoNone),
    /// Will always serialize a [`PoolInfoExtent::Incremental`] field.
    PoolInfoIncremental(GetBlocksResponsePoolInfoIncremental),
    /// Will always serialize a [`PoolInfoExtent::Full`] field.
    PoolInfoFull(GetBlocksResponsePoolInfoFull),
}

impl Default for GetBlocksResponse {
    fn default() -> Self {
        Self::PoolInfoNone(GetBlocksResponsePoolInfoNone::default())
    }
}

/// Common data within all of [`GetBlocksResponse`]'s variants.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetBlocksResponseHeader {
    pub status: Status,
    pub untrusted: bool,
    pub blocks: Vec<BlockCompleteEntry>,
    pub start_height: u64,
    pub current_height: u64,
    pub output_indices: Vec<BlockOutputIndices>,
    pub daemon_time: u64,
}

#[cfg(feature = "epee")]
epee_object! {
    GetBlocksResponseHeader,
    status: Status,
    untrusted: bool,
    blocks: Vec<BlockCompleteEntry>,
    start_height: u64,
    current_height: u64,
    output_indices: Vec<BlockOutputIndices>,
    daemon_time: u64,
}

/// Data within [`GetBlocksResponse::PoolInfoNone`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetBlocksResponsePoolInfoNone {
    /// This field is flattened.
    pub header: GetBlocksResponseHeader,
}

#[cfg(feature = "epee")]
epee_object! {
    GetBlocksResponsePoolInfoNone,
    !flatten: header: GetBlocksResponseHeader,
}

/// Data within [`GetBlocksResponse::PoolInfoIncremental`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetBlocksResponsePoolInfoIncremental {
    pub header: GetBlocksResponseHeader,
    pub added_pool_txs: Vec<PoolTxInfo>,
    pub remaining_added_pool_txids: ByteArrayVec<32>,
    pub removed_pool_txids: ByteArrayVec<32>,
}

#[cfg(feature = "epee")]
epee_object! {
    GetBlocksResponsePoolInfoIncremental,
    added_pool_txs: Vec<PoolTxInfo>,
    remaining_added_pool_txids: ByteArrayVec<32>,
    removed_pool_txids: ByteArrayVec<32>,
    !flatten: header: GetBlocksResponseHeader,
}

/// Data within [`GetBlocksResponse::PoolInfoFull`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetBlocksResponsePoolInfoFull {
    pub header: GetBlocksResponseHeader,
    pub added_pool_txs: Vec<PoolTxInfo>,
    pub remaining_added_pool_txids: ByteArrayVec<32>,
}

#[cfg(feature = "epee")]
epee_object! {
    GetBlocksResponsePoolInfoFull,
    added_pool_txs: Vec<PoolTxInfo>,
    remaining_added_pool_txids: ByteArrayVec<32>,
    !flatten: header: GetBlocksResponseHeader,
}

#[cfg(feature = "epee")]
/// [`EpeeObjectBuilder`] for [`GetBlocksResponse`].
///
/// Not for public usage.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct __GetBlocksResponseEpeeBuilder {
    pub status: Option<Status>,
    pub untrusted: Option<bool>,
    pub blocks: Option<Vec<BlockCompleteEntry>>,
    pub start_height: Option<u64>,
    pub current_height: Option<u64>,
    pub output_indices: Option<Vec<BlockOutputIndices>>,
    pub daemon_time: Option<u64>,
    pub pool_info_extent: Option<PoolInfoExtent>,
    pub added_pool_txs: Option<Vec<PoolTxInfo>>,
    pub remaining_added_pool_txids: Option<ByteArrayVec<32>>,
    pub removed_pool_txids: Option<ByteArrayVec<32>>,
}

#[cfg(feature = "epee")]
impl EpeeObjectBuilder<GetBlocksResponse> for __GetBlocksResponseEpeeBuilder {
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

        // HACK/INVARIANT: this must be manually updated
        // if a field is added to `GetBlocksResponse`.
        read_epee_field! {
            status,
            untrusted,
            blocks,
            start_height,
            current_height,
            output_indices,
            daemon_time,
            pool_info_extent,
            added_pool_txs,
            remaining_added_pool_txids,
            removed_pool_txids
        }

        Ok(true)
    }

    fn finish(self) -> error::Result<GetBlocksResponse> {
        /// The error returned when a required field is missing.
        macro_rules! error {
            ($field_name:literal) => {
                error::Error::Format(concat!(
                    "Required field was not found: ",
                    stringify!($field_name)
                ))
            };
        }

        // INVARIANT:
        // `monerod` omits serializing the field itself when a container is empty,
        // `unwrap_or_default()` is used over `error()` in these cases.
        // Some of the uses are when values have default fallbacks: `daemon_time`, `pool_info_extent`.

        // Deserialize the common fields.
        let header = GetBlocksResponseHeader {
            status: self.status.ok_or(error!("status"))?,
            untrusted: self.untrusted.ok_or(error!("untrusted"))?,
            blocks: self.blocks.unwrap_or_default(),
            start_height: self.start_height.ok_or(error!("start_height"))?,
            current_height: self.current_height.ok_or(error!("current_height"))?,
            output_indices: self.output_indices.unwrap_or_default(),
            daemon_time: self.daemon_time.unwrap_or_default(),
        };

        // Specialize depending on the `pool_info_extent`.
        let pool_info_extent = self.pool_info_extent.unwrap_or_default();
        let this = match pool_info_extent {
            PoolInfoExtent::None => {
                GetBlocksResponse::PoolInfoNone(GetBlocksResponsePoolInfoNone { header })
            }
            PoolInfoExtent::Incremental => {
                GetBlocksResponse::PoolInfoIncremental(GetBlocksResponsePoolInfoIncremental {
                    header,
                    added_pool_txs: self.added_pool_txs.unwrap_or_default(),
                    remaining_added_pool_txids: self.remaining_added_pool_txids.unwrap_or_default(),
                    removed_pool_txids: self.removed_pool_txids.unwrap_or_default(),
                })
            }
            PoolInfoExtent::Full => {
                GetBlocksResponse::PoolInfoFull(GetBlocksResponsePoolInfoFull {
                    header,
                    added_pool_txs: self.added_pool_txs.unwrap_or_default(),
                    remaining_added_pool_txids: self.remaining_added_pool_txids.unwrap_or_default(),
                })
            }
        };

        Ok(this)
    }
}

#[cfg(feature = "epee")]
impl EpeeObject for GetBlocksResponse {
    type Builder = __GetBlocksResponseEpeeBuilder;

    fn number_of_fields(&self) -> u64 {
        // [`PoolInfoExtent`] + inner struct fields.
        let inner_fields = match self {
            Self::PoolInfoNone(s) => s.number_of_fields(),
            Self::PoolInfoIncremental(s) => s.number_of_fields(),
            Self::PoolInfoFull(s) => s.number_of_fields(),
        };

        1 + inner_fields
    }

    fn write_fields<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        const FIELD: &str = "pool_info_extent";

        match self {
            Self::PoolInfoNone(s) => {
                s.write_fields(w)?;
                write_field(PoolInfoExtent::None.to_u8(), FIELD, w)?;
            }
            Self::PoolInfoIncremental(s) => {
                s.write_fields(w)?;
                write_field(PoolInfoExtent::Incremental.to_u8(), FIELD, w)?;
            }
            Self::PoolInfoFull(s) => {
                s.write_fields(w)?;
                write_field(PoolInfoExtent::Full.to_u8(), FIELD, w)?;
            }
        }

        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Request
/// Binary requests.
///
/// This enum contains all [`crate::bin`] requests.
///
/// See also: [`BinResponse`].
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinRequest {
    GetBlocks(GetBlocksRequest),
    GetBlocksByHeight(GetBlocksByHeightRequest),
    GetHashes(GetHashesRequest),
    GetOutputIndexes(GetOutputIndexesRequest),
    GetOuts(GetOutsRequest),
    GetTransactionPoolHashes(GetTransactionPoolHashesRequest),
    GetOutputDistribution(crate::json::GetOutputDistributionRequest),
}

impl RpcCallValue for BinRequest {
    fn is_restricted(&self) -> bool {
        match self {
            Self::GetBlocks(x) => x.is_restricted(),
            Self::GetBlocksByHeight(x) => x.is_restricted(),
            Self::GetHashes(x) => x.is_restricted(),
            Self::GetOutputIndexes(x) => x.is_restricted(),
            Self::GetOuts(x) => x.is_restricted(),
            Self::GetTransactionPoolHashes(x) => x.is_restricted(),
            Self::GetOutputDistribution(x) => x.is_restricted(),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::GetBlocks(x) => x.is_empty(),
            Self::GetBlocksByHeight(x) => x.is_empty(),
            Self::GetHashes(x) => x.is_empty(),
            Self::GetOutputIndexes(x) => x.is_empty(),
            Self::GetOuts(x) => x.is_empty(),
            Self::GetTransactionPoolHashes(x) => x.is_empty(),
            Self::GetOutputDistribution(x) => x.is_empty(),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Response
/// Binary responses.
///
/// This enum contains all [`crate::bin`] responses.
///
/// See also: [`BinRequest`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum BinResponse {
    GetBlocks(GetBlocksResponse),
    GetBlocksByHeight(GetBlocksByHeightResponse),
    GetHashes(GetHashesResponse),
    GetOutputIndexes(GetOutputIndexesResponse),
    GetOuts(GetOutsResponse),
    GetTransactionPoolHashes(GetTransactionPoolHashesResponse),
    GetOutputDistribution(crate::json::GetOutputDistributionResponse),
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
