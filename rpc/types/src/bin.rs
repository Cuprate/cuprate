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
    read_epee_value, write_field, EpeeObject, EpeeObjectBuilder, EpeeValue,
};

use crate::{
    base::{AccessResponseBase, ResponseBase},
    defaults::{default_false, default_height, default_string, default_vec, default_zero},
    free::{is_one, is_zero},
    macros::{define_request, define_request_and_response, define_request_and_response_doc},
    misc::{
        AuxPow, BlockCompleteEntry, BlockHeader, BlockOutputIndices, ChainInfo, ConnectionInfo,
        GetBan, GetOutputsOut, HardforkEntry, HistogramEntry, OutKeyBin, OutputDistributionData,
        Peer, PoolInfoExtent, PoolTxInfo, SetBan, Span, Status, TxBacklogEntry,
    },
};

//---------------------------------------------------------------------------------------------------- TODO
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
        get_transaction_pool_hashesbin,
        cc73fe71162d564ffda8e549b79a350bca53c454,
        core_rpc_server_commands_defs, h, 1593, 1613,
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
    get_transaction_pool_hashesbin,
    cc73fe71162d564ffda8e549b79a350bca53c454,
    core_rpc_server_commands_defs, h, 1593, 1613,
)]
#[allow(dead_code, missing_docs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetBlocksResponse {
    pub status: Status,
    pub untrusted: bool,
    pub blocks: Vec<BlockCompleteEntry>,
    pub start_height: u64,
    pub current_height: u64,
    pub output_indices: Vec<BlockOutputIndices>,
    pub daemon_time: u64,
    pub pool_info_extent: PoolInfoExtent,
    pub added_pool_txs: Vec<PoolTxInfo>,
    pub remaining_added_pool_txids: ByteArrayVec<32>,
    pub removed_pool_txids: ByteArrayVec<32>,
}

#[cfg(feature = "epee")]
/// [`EpeeObjectBuilder`] for [`GetBlocksResponse`].
///
/// Not for public usage.
#[allow(dead_code, missing_docs)]
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
        Ok(GetBlocksResponse {
            status: self.status.ok_or(ELSE)?,
            untrusted: self.untrusted.ok_or(ELSE)?,
            blocks: self.blocks.ok_or(ELSE)?,
            start_height: self.start_height.ok_or(ELSE)?,
            current_height: self.current_height.ok_or(ELSE)?,
            output_indices: self.output_indices.ok_or(ELSE)?,
            daemon_time: self.daemon_time.unwrap_or(0),
            pool_info_extent: self.pool_info_extent.unwrap_or_default(),
            added_pool_txs: self.added_pool_txs.ok_or(ELSE)?,
            remaining_added_pool_txids: self.remaining_added_pool_txids.ok_or(ELSE)?,
            removed_pool_txids: self.removed_pool_txids.ok_or(ELSE)?,
        })
    }
}

#[cfg(feature = "epee")]
impl EpeeObject for GetBlocksResponse {
    type Builder = __GetBlocksResponseEpeeBuilder;

    fn number_of_fields(&self) -> u64 {
        // TODO: fields written depends on branch
        11
    }

    fn write_fields<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        macro_rules! write_field {
            ($($field:ident),*) => {
                $(
                    write_field(self.$field, stringify!($field), w)?;
                )*
            };
        }

        write_field! {
            status,
            untrusted,
            blocks,
            start_height,
            current_height,
            output_indices,
            daemon_time,
            pool_info_extent
        }

        // The following section is why custom epee (de)serialization exists.
        //
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L242-L259>

        if self.pool_info_extent != PoolInfoExtent::None {
            write_field!(added_pool_txs, remaining_added_pool_txids);
        }

        if self.pool_info_extent != PoolInfoExtent::Incremental {
            write_field!(removed_pool_txids);
        }

        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
