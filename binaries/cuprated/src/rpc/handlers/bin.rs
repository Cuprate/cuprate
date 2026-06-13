//! RPC request handler functions (binary endpoints).
//!
//! TODO:
//! Some handlers have `todo!()`s for other Cuprate internals that must be completed, see:
//! <https://github.com/Cuprate/cuprate/pull/355>

use std::num::NonZero;

use anyhow::{anyhow, Error};
use bytes::Bytes;

use cuprate_constants::rpc::{
    GET_BLOCKS_BIN_MAX_BLOCK_COUNT, RESTRICTED_BLOCK_COUNT, RESTRICTED_TRANSACTIONS_COUNT,
};
use cuprate_fixed_bytes::ByteArrayVec;
use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    base::{AccessResponseBase, ResponseBase},
    bin::{
        BinRequest, BinResponse, GetBlocksByHeightRequest, GetBlocksByHeightResponse,
        GetBlocksRequest, GetBlocksResponse, GetHashesRequest, GetHashesResponse,
        GetOutputIndexesRequest, GetOutputIndexesResponse, GetOutsRequest, GetOutsResponse,
        GetTransactionPoolHashesRequest, GetTransactionPoolHashesResponse,
    },
    json::{GetOutputDistributionRequest, GetOutputDistributionResponse},
    misc::{RequestedInfo, Status},
};
use cuprate_types::{
    rpc::{BlockOutputIndices, PoolInfoExtent, TxOutputIndices},
    BlockCompleteEntry,
};

use crate::rpc::{
    handlers::{helper, shared},
    service::{blockchain, txpool},
    CupratedRpcHandler,
};

/// Map a [`BinRequest`] to the function that will lead to a [`BinResponse`].
pub async fn map_request(
    state: CupratedRpcHandler,
    request: BinRequest,
) -> Result<BinResponse, Error> {
    use BinRequest as Req;
    use BinResponse as Resp;

    Ok(match request {
        Req::GetBlocks(r) => Resp::GetBlocks(get_blocks(state, r).await?),
        Req::GetBlocksByHeight(r) => Resp::GetBlocksByHeight(get_blocks_by_height(state, r).await?),
        Req::GetHashes(r) => Resp::GetHashes(get_hashes(state, r).await?),
        Req::GetOutputIndexes(r) => Resp::GetOutputIndexes(get_output_indexes(state, r).await?),
        Req::GetOuts(r) => Resp::GetOuts(get_outs(state, r).await?),
        Req::GetTransactionPoolHashes(r) => {
            Resp::GetTransactionPoolHashes(get_transaction_pool_hashes(state, r).await?)
        }
        Req::GetOutputDistribution(r) => {
            Resp::GetOutputDistribution(get_output_distribution(state, r).await?)
        }
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L611-L789>
async fn get_blocks(
    mut state: CupratedRpcHandler,
    request: GetBlocksRequest,
) -> Result<GetBlocksResponse, Error> {
    // Time should be set early:
    // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L628-L631>
    let daemon_time = cuprate_helper::time::current_unix_timestamp();

    let GetBlocksRequest {
        requested_info,
        block_ids,
        start_height,
        prune,
        no_miner_tx,
        pool_info_since,
        max_block_count,
    } = request;

    let block_hashes: Vec<[u8; 32]> = (&block_ids).into();
    drop(block_ids);

    let Some(requested_info) = RequestedInfo::from_u8(requested_info) else {
        return Err(anyhow!("Wrong requested info"));
    };

    let (get_blocks, get_pool) = match requested_info {
        RequestedInfo::BlocksOnly => (true, false),
        RequestedInfo::BlocksAndPool => (true, true),
        RequestedInfo::PoolOnly => (false, true),
    };

    let (pool_info_extent, added_pool_txs, remaining_added_pool_txids, removed_pool_txids);

    if get_pool {
        let is_restricted = state.is_restricted();
        let max_tx_count = if is_restricted {
            RESTRICTED_TRANSACTIONS_COUNT
        } else {
            usize::MAX
        };

        let pool_since =
            txpool::pool_info_since(&state.tx_handler.txpool_manager, pool_info_since).await?;

        let (to_send, remaining): (&[[u8; 32]], &[[u8; 32]]) =
            if pool_since.added.len() > max_tx_count {
                pool_since.added.split_at(max_tx_count)
            } else {
                (&pool_since.added, &[])
            };

        added_pool_txs = txpool::tx_blobs_by_hash(&mut state.txpool_read, to_send, prune).await?;
        remaining_added_pool_txids = remaining.to_vec().into();
        removed_pool_txids = pool_since.removed.into();

        pool_info_extent = if pool_since.full_required {
            PoolInfoExtent::Full
        } else {
            PoolInfoExtent::Incremental
        };
    } else {
        pool_info_extent = PoolInfoExtent::None;
        added_pool_txs = vec![];
        remaining_added_pool_txids = ByteArrayVec::default();
        removed_pool_txids = ByteArrayVec::default();
    }

    let resp = GetBlocksResponse {
        base: helper::access_response_base(false),
        blocks: vec![],
        start_height: 0,
        current_height: 0,
        top_block_hash: [0; 32],
        output_indices: vec![],
        daemon_time,
        pool_info_extent,
        added_pool_txs,
        remaining_added_pool_txids,
        removed_pool_txids,
    };

    if !get_blocks {
        return Ok(resp);
    }

    let len = u64_to_usize(if max_block_count > 0 {
        max_block_count.min(GET_BLOCKS_BIN_MAX_BLOCK_COUNT)
    } else {
        GET_BLOCKS_BIN_MAX_BLOCK_COUNT
    });

    let req_start_height = if start_height > 0 {
        Some(u64_to_usize(start_height))
    } else {
        None
    };

    if req_start_height.is_none() && block_hashes.is_empty() {
        return Ok(GetBlocksResponse {
            base: AccessResponseBase {
                response_base: ResponseBase {
                    status: Status::Failed,
                    untrusted: false,
                },
                credits: 0,
                top_hash: String::new(),
            },
            ..resp
        });
    }

    let (top_height, top_hash) = helper::top_height(&mut state);
    if start_height > top_height || block_hashes.first() == Some(&top_hash) {
        return Ok(GetBlocksResponse {
            current_height: top_height + 1,
            top_block_hash: top_hash,
            ..resp
        });
    }

    let (blocks, chain_height, actual_start_height, output_indices, top_hash) =
        blockchain::block_complete_entries_above_split_point(
            &mut state.blockchain_read,
            block_hashes,
            req_start_height,
            no_miner_tx,
            len,
            prune,
        )
        .await?;

    Ok(GetBlocksResponse {
        blocks,
        current_height: usize_to_u64(chain_height),
        start_height: usize_to_u64(actual_start_height),
        top_block_hash: top_hash,
        output_indices: output_indices
            .into_iter()
            .map(|block| BlockOutputIndices {
                indices: block
                    .into_iter()
                    .map(|indices| TxOutputIndices { indices })
                    .collect(),
            })
            .collect(),
        ..resp
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L817-L857>
async fn get_blocks_by_height(
    mut state: CupratedRpcHandler,
    request: GetBlocksByHeightRequest,
) -> Result<GetBlocksByHeightResponse, Error> {
    if state.is_restricted() && request.heights.len() > RESTRICTED_BLOCK_COUNT {
        return Err(anyhow!("Too many blocks requested in restricted mode"));
    }

    let blocks =
        blockchain::block_complete_entries_by_height(&mut state.blockchain_read, request.heights)
            .await?;

    Ok(GetBlocksByHeightResponse {
        base: helper::access_response_base(false),
        blocks,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L859-L880>
async fn get_hashes(
    mut state: CupratedRpcHandler,
    request: GetHashesRequest,
) -> Result<GetHashesResponse, Error> {
    let GetHashesRequest {
        start_height: _,
        block_ids,
    } = request;

    // monerod ignores `req.start_height` it's overwritten by the split point derived from `block_ids`.
    // https://github.com/monero-project/monero/blob/495c7f18dd92c53753339a9aff65e5ae097958b4/src/rpc/core_rpc_server.cpp#L894
    // https://github.com/monero-project/monero/blob/495c7f18dd92c53753339a9aff65e5ae097958b4/src/cryptonote_core/blockchain.cpp#L2736
    // https://github.com/monero-project/monero/blob/495c7f18dd92c53753339a9aff65e5ae097958b4/src/cryptonote_core/blockchain.cpp#L2487
    let hashes: Vec<[u8; 32]> = (&block_ids).into();

    let (m_block_ids, start, current_height) =
        blockchain::next_chain_entry(&mut state.blockchain_read, hashes).await?;

    let Some(start) = start else {
        return Err(anyhow!("Could not find split point."));
    };

    Ok(GetHashesResponse {
        base: helper::access_response_base(false),
        m_block_ids: m_block_ids.into(),
        current_height: usize_to_u64(current_height),
        start_height: usize_to_u64(start),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L959-L977>
async fn get_output_indexes(
    mut state: CupratedRpcHandler,
    request: GetOutputIndexesRequest,
) -> Result<GetOutputIndexesResponse, Error> {
    Ok(GetOutputIndexesResponse {
        base: helper::access_response_base(false),
        o_indexes: blockchain::tx_output_indexes(&mut state.blockchain_read, request.txid).await?,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L882-L910>
async fn get_outs(
    state: CupratedRpcHandler,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, Error> {
    shared::get_outs(state, request).await
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1689-L1711>
async fn get_transaction_pool_hashes(
    mut state: CupratedRpcHandler,
    _: GetTransactionPoolHashesRequest,
) -> Result<GetTransactionPoolHashesResponse, Error> {
    Ok(GetTransactionPoolHashesResponse {
        base: helper::access_response_base(false),
        tx_hashes: shared::get_transaction_pool_hashes(state)
            .await
            .map(ByteArrayVec::from)?,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3352-L3398>
async fn get_output_distribution(
    state: CupratedRpcHandler,
    request: GetOutputDistributionRequest,
) -> Result<GetOutputDistributionResponse, Error> {
    // monerod rejects non-binary requests on the binary endpoint:
    // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3422>
    if !request.binary {
        return Err(anyhow!("Binary only call"));
    }

    shared::get_output_distribution(state, request).await
}
