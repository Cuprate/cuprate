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

#[doc = define_request_and_response_doc!(
    "request" => GetBlocksRequest,
    get_blocksbin,
    cc73fe71162d564ffda8e549b79a350bca53c454,
    core_rpc_server_commands_defs, h, 162, 262,
)]
///
/// This response's variant depends upon [`crate::misc::PoolInfoExtent`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GetBlocksResponse {
    /// Will always serialize a [`crate::misc::PoolInfoExtent::None`] field.
    PoolInfoNone(GetBlocksResponsePoolInfoNone),
    /// Will always serialize a [`crate::misc::PoolInfoExtent::Incremental`] field.
    PoolInfoIncremental(GetBlocksResponsePoolInfoIncremental),
    /// Will always serialize a [`crate::misc::PoolInfoExtent::Full`] field.
    PoolInfoFull(GetBlocksResponsePoolInfoFull),
}

impl Default for GetBlocksResponse {
    fn default() -> Self {
        Self::PoolInfoNone(GetBlocksResponsePoolInfoNone::default())
    }
}

/// Data within [`GetBlocksResponse::PoolInfoNone`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetBlocksResponsePoolInfoNone {
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
    GetBlocksResponsePoolInfoNone,
    status: Status,
    untrusted: bool,
    blocks: Vec<BlockCompleteEntry>,
    start_height: u64,
    current_height: u64,
    output_indices: Vec<BlockOutputIndices>,
    daemon_time: u64,
}

/// Data within [`GetBlocksResponse::PoolInfoIncremental`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetBlocksResponsePoolInfoIncremental {
    pub status: Status,
    pub untrusted: bool,
    pub blocks: Vec<BlockCompleteEntry>,
    pub start_height: u64,
    pub current_height: u64,
    pub output_indices: Vec<BlockOutputIndices>,
    pub daemon_time: u64,
    pub added_pool_txs: Vec<PoolTxInfo>,
    pub remaining_added_pool_txids: ByteArrayVec<32>,
    pub removed_pool_txids: ByteArrayVec<32>,
}

#[cfg(feature = "epee")]
epee_object! {
    GetBlocksResponsePoolInfoIncremental,
    status: Status,
    untrusted: bool,
    blocks: Vec<BlockCompleteEntry>,
    start_height: u64,
    current_height: u64,
    output_indices: Vec<BlockOutputIndices>,
    daemon_time: u64,
    added_pool_txs: Vec<PoolTxInfo>,
    remaining_added_pool_txids: ByteArrayVec<32>,
    removed_pool_txids: ByteArrayVec<32>,
}

/// Data within [`GetBlocksResponse::PoolInfoFull`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetBlocksResponsePoolInfoFull {
    pub status: Status,
    pub untrusted: bool,
    pub blocks: Vec<BlockCompleteEntry>,
    pub start_height: u64,
    pub current_height: u64,
    pub output_indices: Vec<BlockOutputIndices>,
    pub daemon_time: u64,
    pub added_pool_txs: Vec<PoolTxInfo>,
    pub remaining_added_pool_txids: ByteArrayVec<32>,
}

#[cfg(feature = "epee")]
epee_object! {
    GetBlocksResponsePoolInfoFull,
    status: Status,
    untrusted: bool,
    blocks: Vec<BlockCompleteEntry>,
    start_height: u64,
    current_height: u64,
    output_indices: Vec<BlockOutputIndices>,
    daemon_time: u64,
    added_pool_txs: Vec<PoolTxInfo>,
    remaining_added_pool_txids: ByteArrayVec<32>,
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
    pub pool_info_extent: Option<crate::misc::PoolInfoExtent>,
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
        const ELSE: error::Error = error::Error::Format("Required field was not found!");

        let status = self.status.ok_or(ELSE)?;
        let untrusted = self.untrusted.ok_or(ELSE)?;
        let blocks = self.blocks.ok_or(ELSE)?;
        let start_height = self.start_height.ok_or(ELSE)?;
        let current_height = self.current_height.ok_or(ELSE)?;
        let output_indices = self.output_indices.ok_or(ELSE)?;
        let daemon_time = self.daemon_time.ok_or(ELSE)?;
        let pool_info_extent = self.pool_info_extent.ok_or(ELSE)?;

        let this = match pool_info_extent {
            crate::misc::PoolInfoExtent::None => {
                GetBlocksResponse::PoolInfoNone(GetBlocksResponsePoolInfoNone {
                    status,
                    untrusted,
                    blocks,
                    start_height,
                    current_height,
                    output_indices,
                    daemon_time,
                })
            }
            crate::misc::PoolInfoExtent::Incremental => {
                GetBlocksResponse::PoolInfoIncremental(GetBlocksResponsePoolInfoIncremental {
                    status,
                    untrusted,
                    blocks,
                    start_height,
                    current_height,
                    output_indices,
                    daemon_time,
                    added_pool_txs: self.added_pool_txs.ok_or(ELSE)?,
                    remaining_added_pool_txids: self.remaining_added_pool_txids.ok_or(ELSE)?,
                    removed_pool_txids: self.removed_pool_txids.ok_or(ELSE)?,
                })
            }
            crate::misc::PoolInfoExtent::Full => {
                GetBlocksResponse::PoolInfoFull(GetBlocksResponsePoolInfoFull {
                    status,
                    untrusted,
                    blocks,
                    start_height,
                    current_height,
                    output_indices,
                    daemon_time,
                    added_pool_txs: self.added_pool_txs.ok_or(ELSE)?,
                    remaining_added_pool_txids: self.remaining_added_pool_txids.ok_or(ELSE)?,
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
        // [`crate::misc::PoolInfoExtent`] + inner struct fields.
        let inner_fields = match self {
            Self::PoolInfoNone(s) => s.number_of_fields(),
            Self::PoolInfoIncremental(s) => s.number_of_fields(),
            Self::PoolInfoFull(s) => s.number_of_fields(),
        };

        1 + inner_fields
    }

    fn write_fields<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        match self {
            Self::PoolInfoNone(s) => {
                s.write_fields(w)?;
                write_field(
                    crate::misc::PoolInfoExtent::None.to_u8(),
                    "pool_info_extent",
                    w,
                )?;
            }
            Self::PoolInfoIncremental(s) => {
                s.write_fields(w)?;
                write_field(
                    crate::misc::PoolInfoExtent::Incremental.to_u8(),
                    "pool_info_extent",
                    w,
                )?;
            }
            Self::PoolInfoFull(s) => {
                s.write_fields(w)?;
                write_field(
                    crate::misc::PoolInfoExtent::Full.to_u8(),
                    "pool_info_extent",
                    w,
                )?;
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
