//! Binary types from [`.bin` endpoints](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin).
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
use cuprate_fixed_bytes::ByteArrayVec;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::container_as_blob::ContainerAsBlob;

use cuprate_types::BlockCompleteEntry;

use crate::{
    base::AccessResponseBase,
    macros::define_request_and_response,
    misc::{BlockOutputIndices, GetOutputsOut, OutKeyBin, PoolInfo},
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

define_request_and_response! {
    get_blocksbin,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 162..=262,

    GetBlocks,

    Request {
        requested_info: u8 = default_zero::<u8>(), "default_zero",
        // FIXME: This is a `std::list` in `monerod` because...?
        block_ids: ByteArrayVec<32>,
        start_height: u64,
        prune: bool,
        no_miner_tx: bool = default_false(), "default_false",
        pool_info_since: u64 = default_zero::<u64>(), "default_zero",
    },

    // TODO: add `top_block_hash` field
    // <https://github.com/monero-project/monero/blame/893916ad091a92e765ce3241b94e706ad012b62a/src/rpc/core_rpc_server_commands_defs.h#L263>
    AccessResponseBase {
        blocks: Vec<BlockCompleteEntry>,
        start_height: u64,
        current_height: u64,
        output_indices: Vec<BlockOutputIndices>,
        daemon_time: u64 = default_zero::<u64>(), "default_zero",
        pool_info: PoolInfo = PoolInfo::None, "PoolInfo::default",
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
