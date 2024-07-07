//! Binary types from [binary](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin) endpoints.
//!
//! Most (if not all) of these types are defined here:
//! - <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h>

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    base::{AccessResponseBase, ResponseBase},
    defaults::{
        default_bool, default_height, default_string, default_u64, default_u8, default_vec,
    },
    free::{is_one, is_zero},
    macros::define_request_and_response,
    misc::{
        AuxPow, BlockCompleteEntry, BlockHeader, BlockOutputIndices, ChainInfo, ConnectionInfo,
        GetBan, HardforkEntry, HistogramEntry, OutputDistributionData, Peer, PoolTxInfo, SetBan,
        Span, Status, TxBacklogEntry,
    },
};

//---------------------------------------------------------------------------------------------------- TODO
define_request_and_response! {
    get_blocks_bin,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 162..=262,
    GetBlocksBin,
    Request {
        #[cfg_attr(feature = "serde", serde(default = "default_u8"))]
        requested_info: u8 = default_u8(),
        // TODO: This is a `std::list` in `monerod` because...?
        block_ids: Vec<[u8; 32]>,
        start_height: u64,
        prune: bool,
        #[cfg_attr(feature = "serde", serde(default = "default_bool"))]
        no_miner_tx: bool = default_bool(),
        #[cfg_attr(feature = "serde", serde(default = "default_u64"))]
        pool_info_since: u64 = default_u64(),
    },
    ResponseBase {
      blocks: Vec<BlockCompleteEntry>,
      start_height: u64,
      current_height: u64,
      output_indices: Vec<BlockOutputIndices>,
      daemon_time: u64,
      pool_info_extent: u8,
      added_pool_txs: Vec<PoolTxInfo>,
      remaining_added_pool_txids: Vec<[u8; 32]>,
      removed_pool_txids: Vec<[u8; 32]>,
    }
}

define_request_and_response! {
    add_aux_pow,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1068..=1112,
    AddAuxPow,
    Request {
        blocktemplate_blob: String,
        aux_pow: Vec<AuxPow>,
    },
    ResponseBase {
        blocktemplate_blob: String,
        blockhashing_blob: String,
        merkle_root: String,
        merkle_tree_depth: u64,
        aux_pow: Vec<AuxPow>,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
