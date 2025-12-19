//! RPC request handler functions (binary endpoints).
//!
//! TODO:
//! Some handlers have `todo!()`s for other Cuprate internals that must be completed, see:
//! <https://github.com/Cuprate/cuprate/pull/355>

use std::num::NonZero;

use anyhow::{anyhow, Error};
use bytes::Bytes;

use crate::rpc::{
    handlers::{helper, shared, shared::not_available},
    service::{blockchain, txpool},
    CupratedRpcHandler,
};
use cuprate_constants::rpc::{RESTRICTED_BLOCK_COUNT, RESTRICTED_TRANSACTIONS_COUNT};
use cuprate_fixed_bytes::ByteArrayVec;
use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    base::{AccessResponseBase, ResponseBase},
    bin::{
        BinRequest, BinResponse, GetBlocksByHeightRequest, GetBlocksByHeightResponse,
        GetBlocksRequest, GetBlocksResponse, GetHashesRequest, GetHashesResponse,
        GetOutputIndexesRequest, GetOutputIndexesResponse, GetOutsRequest, GetOutsResponse,
    },
    json::{GetOutputDistributionRequest, GetOutputDistributionResponse},
    misc::RequestedInfo,
};
use cuprate_types::rpc::{PoolInfoFull, PoolInfoIncremental};
use cuprate_types::{
    rpc::{BlockOutputIndices, PoolInfo, PoolInfoExtent, TxOutputIndices},
    BlockCompleteEntry,
};

/// Map a [`BinRequest`] to the function that will lead to a [`BinResponse`].
pub async fn map_request(
    state: CupratedRpcHandler,
    request: BinRequest,
) -> Result<BinResponse, Error> {
    use BinRequest as Req;
    use BinResponse as Resp;

    Ok(match request {
        Req::GetBlocks(r) => Resp::GetBlocks(get_blocks(state, r).await.unwrap()),
        Req::GetBlocksByHeight(r) => Resp::GetBlocksByHeight(not_available()?),
        Req::GetHashes(r) => Resp::GetHashes(get_hashes(state, r).await?),
        Req::GetOutputIndexes(r) => Resp::GetOutputIndexes(not_available()?),
        Req::GetOuts(r) => Resp::GetOuts(not_available()?),
        Req::GetOutputDistribution(r) => Resp::GetOutputDistribution(not_available()?),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L611-L789>
async fn get_blocks(
    mut state: CupratedRpcHandler,
    request: GetBlocksRequest,
) -> Result<GetBlocksResponse, Error> {
    tracing::info!("Get blocks");

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
    } = request;

    let block_hashes: Vec<[u8; 32]> = (&block_ids).into();
    drop(block_ids);

    let Some(requested_info) = RequestedInfo::from_u8(request.requested_info) else {
        return Err(anyhow!("Wrong requested info"));
    };

    let (get_blocks, get_pool) = match requested_info {
        RequestedInfo::BlocksOnly => (true, false),
        RequestedInfo::BlocksAndPool => (true, true),
        RequestedInfo::PoolOnly => (false, true),
    };

    let mut pool_info_extent = PoolInfoExtent::None;

    let pool_info = if get_pool {
        /*
        let is_restricted = state.is_restricted();
        let include_sensitive_txs = !is_restricted;

        let max_tx_count = if is_restricted {
            RESTRICTED_TRANSACTIONS_COUNT
        } else {
            usize::MAX
        };

        txpool::pool_info(
            &mut state.txpool_read,
            include_sensitive_txs,
            max_tx_count,
            NonZero::new(u64_to_usize(request.pool_info_since)),
        )
        .await?
        */
        if request.pool_info_since == 0 {
            pool_info_extent = PoolInfoExtent::Full;
        } else {
            pool_info_extent = PoolInfoExtent::Incremental;
        }
    } else {
        pool_info_extent = PoolInfoExtent::None;
    };

    let resp = GetBlocksResponse {
        base: helper::access_response_base(false),
        blocks: vec![],
        start_height: 0,
        current_height: 0,
        output_indices: vec![],
        daemon_time,
        pool_info_extent,
        added_pool_txs: vec![],
        remaining_added_pool_txids: Default::default(),
        removed_pool_txids: Default::default(),
    };

    if !get_blocks {
        return Ok(resp);
    }


    if let Some(block_id) = block_hashes.first() {
        let (height, hash) = helper::top_height(&mut state).await?;

        if hash == *block_id {
            return Ok(GetBlocksResponse {
                current_height: height + 1,
                ..resp
            });
        }
    }

    let (blocks, height, start_height, output_indices) =
        blockchain::block_complete_entries_above_split_point(
            &mut state.blockchain_read,
            block_hashes,
            true,
            prune,
        )
        .await?;

    tracing::info!("Got blocks");

    //tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    //tracing::trace!("blocks: {:?}", blocks);
    tracing::trace!("outputs: {:?}", output_indices.len());

    Ok(GetBlocksResponse {
        blocks,
        current_height: usize_to_u64(height),
        start_height: usize_to_u64(start_height),
        output_indices: output_indices
            .into_iter()
            .map(|i| BlockOutputIndices {
                indices: i
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
        start_height,
        block_ids,
    } = request;

    let hashes: Vec<[u8; 32]> = (&block_ids).into();

    let (m_blocks_ids, start, current_height) =
        blockchain::next_chain_entry(&mut state.blockchain_read, hashes, start_height).await?;

    if start.is_none() {
        return Err(anyhow!("Could not find split point."));
    }

    Ok(GetHashesResponse {
        base: helper::access_response_base(false),
        m_blocks_ids: m_blocks_ids.into(),
        current_height: usize_to_u64(current_height),
        start_height,
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

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3352-L3398>
async fn get_output_distribution(
    state: CupratedRpcHandler,
    request: GetOutputDistributionRequest,
) -> Result<GetOutputDistributionResponse, Error> {
    shared::get_output_distribution(state, request).await
}
