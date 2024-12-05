//! RPC request handler functions (binary endpoints).

use anyhow::{anyhow, Error};

use cuprate_constants::rpc::{RESTRICTED_BLOCK_COUNT, RESTRICTED_TRANSACTIONS_COUNT};
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
use cuprate_types::BlockCompleteEntry;

use crate::rpc::{helper, request::blockchain, CupratedRpcHandler};

/// Map a [`BinRequest`] to the function that will lead to a [`BinResponse`].
pub(super) async fn map_request(
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
    state: CupratedRpcHandler,
    request: GetBlocksRequest,
) -> Result<GetBlocksResponse, Error> {
    // Time should be set early:
    // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L628-L631>
    let daemon_time = cuprate_helper::time::current_unix_timestamp();

    let Some(requested_info) = RequestedInfo::from_u8(request.requested_info) else {
        return Err(anyhow!("Failed, wrong requested info"));
    };

    let (get_blocks, get_pool) = match requested_info {
        RequestedInfo::BlocksOnly => (true, false),
        RequestedInfo::BlocksAndPool => (true, true),
        RequestedInfo::PoolOnly => (false, true),
    };

    if get_pool {
        let max_tx_count = if state.is_restricted() {
            RESTRICTED_TRANSACTIONS_COUNT
        } else {
            usize::MAX
        };

        todo!();
    }

    if get_blocks {
        if !request.block_ids.is_empty() {
            todo!();
        }

        todo!();
    }

    // Ok(GetBlocksResponse {
    //     base: ResponseBase::OK,
    //     ..todo!()
    // })
    Ok(todo!())
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L817-L857>
async fn get_blocks_by_height(
    state: CupratedRpcHandler,
    request: GetBlocksByHeightRequest,
) -> Result<GetBlocksByHeightResponse, Error> {
    if state.is_restricted() && request.heights.len() > RESTRICTED_BLOCK_COUNT {
        return Err(anyhow!("Too many blocks requested in restricted mode"));
    }

    let blocks = request
        .heights
        .into_iter()
        .map(|height| Ok(todo!()))
        .collect::<Result<Vec<BlockCompleteEntry>, Error>>()?;

    Ok(GetBlocksByHeightResponse {
        base: AccessResponseBase::OK,
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

    const GENESIS_BLOCK_HASH: [u8; 32] = [0; 32]; // TODO
    if last != GENESIS_BLOCK_HASH {
        return Err(anyhow!(
            "genesis block mismatch, found: {last:?}, expected: {GENESIS_BLOCK_HASH:?}"
        ));
    }

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
        base: AccessResponseBase::OK,
        m_blocks_ids,
        start_height,
        current_height,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L959-L977>
async fn get_output_indexes(
    state: CupratedRpcHandler,
    request: GetOutputIndexesRequest,
) -> Result<GetOutputIndexesResponse, Error> {
    Ok(GetOutputIndexesResponse {
        base: AccessResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L882-L910>
async fn get_outs(
    state: CupratedRpcHandler,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, Error> {
    Ok(GetOutsResponse {
        base: AccessResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1689-L1711>
async fn get_transaction_pool_hashes(
    state: CupratedRpcHandler,
    request: GetTransactionPoolHashesRequest,
) -> Result<GetTransactionPoolHashesResponse, Error> {
    Ok(GetTransactionPoolHashesResponse {
        base: AccessResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3352-L3398>
async fn get_output_distribution(
    state: CupratedRpcHandler,
    request: GetOutputDistributionRequest,
) -> Result<GetOutputDistributionResponse, Error> {
    Ok(GetOutputDistributionResponse {
        base: AccessResponseBase::OK,
        ..todo!()
    })
}
