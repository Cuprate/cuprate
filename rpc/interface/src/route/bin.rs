//! Binary types from [`.bin` endpoints](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin).
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- TODO
// define_request_and_response! {
//     get_blocksbin,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 162..=262,
//     GetBlocks,
//     Request {
//         requested_info: u8 = default_zero(), "default_zero",
//         // FIXME: This is a `std::list` in `monerod` because...?
//         block_ids: ByteArrayVec<32>,
//         start_height: u64,
//         prune: bool,
//         no_miner_tx: bool = default_false(), "default_false",
//         pool_info_since: u64 = default_zero(), "default_zero",
//     },
//     // TODO: this has custom epee (de)serialization.
//     // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L242-L259>
//     ResponseBase {
//         blocks: Vec<BlockCompleteEntry>,
//         start_height: u64,
//         current_height: u64,
//         output_indices: Vec<BlockOutputIndices>,
//         daemon_time: u64,
//         pool_info_extent: u8,
//         added_pool_txs: Vec<PoolTxInfo>,
//         remaining_added_pool_txids: Vec<[u8; 32]>,
//         removed_pool_txids: Vec<[u8; 32]>,
//     }
// }

// define_request_and_response! {
//     get_blocks_by_heightbin,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 264..=286,
//     GetBlocksByHeight,
//     Request {
//         heights: Vec<u64>,
//     },
//     AccessResponseBase {
//         blocks: Vec<BlockCompleteEntry>,
//     }
// }

// define_request_and_response! {
//     get_hashesbin,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 309..=338,
//     GetHashes,
//     Request {
//         block_ids: ByteArrayVec<32>,
//         start_height: u64,
//     },
//     AccessResponseBase {
//         m_blocks_ids: ByteArrayVec<32>,
//         start_height: u64,
//         current_height: u64,
//     }
// }

// #[cfg(not(feature = "epee"))]
// define_request_and_response! {
//     get_o_indexesbin,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 487..=510,
//     GetOutputIndexes,
//     #[derive(Copy)]
//     Request {
//         txid: [u8; 32],
//     },
//     AccessResponseBase {
//         o_indexes: Vec<u64>,
//     }
// }

// #[cfg(feature = "epee")]
// define_request_and_response! {
//     get_o_indexesbin,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 487..=510,
//     GetOutputIndexes,
//     #[derive(Copy)]
//     Request {
//         txid: [u8; 32],
//     },
//     AccessResponseBase {
//         o_indexes: Vec<u64> as ContainerAsBlob<u64>,
//     }
// }

// define_request_and_response! {
//     get_outsbin,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 512..=565,
//     GetOuts,
//     Request {
//         outputs: Vec<GetOutputsOut>,
//         get_txid: bool = default_false(), "default_false",
//     },
//     AccessResponseBase {
//         outs: Vec<OutKeyBin>,
//     }
// }

// define_request_and_response! {
//     get_transaction_pool_hashesbin,
//     cc73fe71162d564ffda8e549b79a350bca53c454 =>
//     core_rpc_server_commands_defs.h => 1593..=1613,
//     GetTransactionPoolHashes,
//     Request {},
//     AccessResponseBase {
//         tx_hashes: ByteArrayVec<32>,
//     }
// }

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
