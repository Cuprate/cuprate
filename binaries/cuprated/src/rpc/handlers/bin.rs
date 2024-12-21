//! RPC request handler functions (binary endpoints).
//!
//! TODO:
//! Some handlers have `todo!()`s for other Cuprate internals that must be completed, see:
//! <https://github.com/Cuprate/cuprate/pull/355>

use std::num::NonZero;

use anyhow::{anyhow, Error};
use bytes::Bytes;

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
        GetTransactionPoolHashesRequest, GetTransactionPoolHashesResponse,
    },
    json::{GetOutputDistributionRequest, GetOutputDistributionResponse},
    misc::RequestedInfo,
};
use cuprate_types::{
    rpc::{PoolInfo, PoolInfoExtent},
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

    let Some(requested_info) = RequestedInfo::from_u8(request.requested_info) else {
        return Err(anyhow!("Wrong requested info"));
    };

    let (get_blocks, get_pool) = match requested_info {
        RequestedInfo::BlocksOnly => (true, false),
        RequestedInfo::BlocksAndPool => (true, true),
        RequestedInfo::PoolOnly => (false, true),
    };

    let pool_info_extent = PoolInfoExtent::None;

    let pool_info = if get_pool {
        let include_sensitive_txs = !state.is_restricted();
        let max_tx_count = if state.is_restricted() {
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
    } else {
        PoolInfo::None
    };

    let resp = GetBlocksResponse {
        base: helper::access_response_base(false),
        blocks: vec![],
        start_height: 0,
        current_height: 0,
        output_indices: vec![],
        daemon_time,
        pool_info,
    };

    if !get_blocks {
        return Ok(resp);
    }

    // FIXME: impl `first()`
    if !request.block_ids.is_empty() {
        let block_id = request.block_ids[0];

        let (height, hash) = helper::top_height(&mut state).await?;

        if hash == block_id {
            return Ok(GetBlocksResponse {
                current_height: height + 1,
                ..resp
            });
        }
    }

    let block_hashes: Vec<[u8; 32]> = (&request.block_ids).into();

    let (blocks, missing_hashes, blockchain_height) =
        blockchain::block_complete_entries(&mut state.blockchain_read, block_hashes).await?;

    if !missing_hashes.is_empty() {
        return Err(anyhow!("Missing blocks"));
    }

    Ok(GetBlocksResponse {
        blocks,
        current_height: usize_to_u64(blockchain_height),
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
    // FIXME: impl `last()`
    let last = {
        let len = request.block_ids.len();

        if len == 0 {
            return Err(anyhow!("block_ids empty"));
        }

        request.block_ids[len - 1]
    };

    let mut bytes = request.block_ids;
    let hashes: Vec<[u8; 32]> = (&bytes).into();

    let (current_height, _) = helper::top_height(&mut state).await?;

    let Some((index, start_height)) =
        blockchain::find_first_unknown(&mut state.blockchain_read, hashes).await?
    else {
        return Err(anyhow!("Failed"));
    };

    let m_blocks_ids = bytes.split_off(index);

    Ok(GetHashesResponse {
        base: helper::access_response_base(false),
        m_blocks_ids,
        start_height,
        current_height,
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
    shared::get_output_distribution(state, request).await
}
