//! RPC request handler functions (other JSON endpoints).

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Error};

use cuprate_constants::rpc::{
    MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT, RESTRICTED_SPENT_KEY_IMAGES_COUNT,
};
use cuprate_helper::cast::usize_to_u64;
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    base::{AccessResponseBase, ResponseBase},
    misc::{KeyImageSpentStatus, Status},
    other::{
        GetAltBlocksHashesRequest, GetAltBlocksHashesResponse, GetHeightRequest, GetHeightResponse,
        GetLimitRequest, GetLimitResponse, GetNetStatsRequest, GetNetStatsResponse, GetOutsRequest,
        GetOutsResponse, GetPeerListRequest, GetPeerListResponse, GetPublicNodesRequest,
        GetPublicNodesResponse, GetTransactionPoolHashesRequest, GetTransactionPoolHashesResponse,
        GetTransactionPoolRequest, GetTransactionPoolResponse, GetTransactionPoolStatsRequest,
        GetTransactionPoolStatsResponse, GetTransactionsRequest, GetTransactionsResponse,
        InPeersRequest, InPeersResponse, IsKeyImageSpentRequest, IsKeyImageSpentResponse,
        MiningStatusRequest, MiningStatusResponse, OtherRequest, OtherResponse, OutPeersRequest,
        OutPeersResponse, PopBlocksRequest, PopBlocksResponse, SaveBcRequest, SaveBcResponse,
        SendRawTransactionRequest, SendRawTransactionResponse, SetBootstrapDaemonRequest,
        SetBootstrapDaemonResponse, SetLimitRequest, SetLimitResponse, SetLogCategoriesRequest,
        SetLogCategoriesResponse, SetLogHashRateRequest, SetLogHashRateResponse,
        SetLogLevelRequest, SetLogLevelResponse, StartMiningRequest, StartMiningResponse,
        StopDaemonRequest, StopDaemonResponse, StopMiningRequest, StopMiningResponse,
        UpdateRequest, UpdateResponse,
    },
};
use cuprate_types::rpc::OutKey;

use crate::{
    rpc::CupratedRpcHandler,
    rpc::{helper, request::blockchain},
};

use super::request::blockchain_manager;

/// Map a [`OtherRequest`] to the function that will lead to a [`OtherResponse`].
pub(super) async fn map_request(
    state: CupratedRpcHandler,
    request: OtherRequest,
) -> Result<OtherResponse, Error> {
    use OtherRequest as Req;
    use OtherResponse as Resp;

    Ok(match request {
        Req::GetHeight(r) => Resp::GetHeight(get_height(state, r).await?),
        Req::GetTransactions(r) => Resp::GetTransactions(get_transactions(state, r).await?),
        Req::GetAltBlocksHashes(r) => {
            Resp::GetAltBlocksHashes(get_alt_blocks_hashes(state, r).await?)
        }
        Req::IsKeyImageSpent(r) => Resp::IsKeyImageSpent(is_key_image_spent(state, r).await?),
        Req::SendRawTransaction(r) => {
            Resp::SendRawTransaction(send_raw_transaction(state, r).await?)
        }
        Req::SaveBc(r) => Resp::SaveBc(save_bc(state, r).await?),
        Req::GetPeerList(r) => Resp::GetPeerList(get_peer_list(state, r).await?),
        Req::SetLogLevel(r) => Resp::SetLogLevel(set_log_level(state, r).await?),
        Req::SetLogCategories(r) => Resp::SetLogCategories(set_log_categories(state, r).await?),
        Req::SetBootstrapDaemon(r) => {
            Resp::SetBootstrapDaemon(set_bootstrap_daemon(state, r).await?)
        }
        Req::GetTransactionPool(r) => {
            Resp::GetTransactionPool(get_transaction_pool(state, r).await?)
        }
        Req::GetTransactionPoolStats(r) => {
            Resp::GetTransactionPoolStats(get_transaction_pool_stats(state, r).await?)
        }
        Req::StopDaemon(r) => Resp::StopDaemon(stop_daemon(state, r).await?),
        Req::GetLimit(r) => Resp::GetLimit(get_limit(state, r).await?),
        Req::SetLimit(r) => Resp::SetLimit(set_limit(state, r).await?),
        Req::OutPeers(r) => Resp::OutPeers(out_peers(state, r).await?),
        Req::InPeers(r) => Resp::InPeers(in_peers(state, r).await?),
        Req::GetNetStats(r) => Resp::GetNetStats(get_net_stats(state, r).await?),
        Req::GetOuts(r) => Resp::GetOuts(get_outs(state, r).await?),
        Req::Update(r) => Resp::Update(update(state, r).await?),
        Req::PopBlocks(r) => Resp::PopBlocks(pop_blocks(state, r).await?),
        Req::GetTransactionPoolHashes(r) => {
            Resp::GetTransactionPoolHashes(get_transaction_pool_hashes(state, r).await?)
        }
        Req::GetPublicNodes(r) => Resp::GetPublicNodes(get_public_nodes(state, r).await?),

        // Unsupported requests.
        Req::StartMining(_)
        | Req::StopMining(_)
        | Req::MiningStatus(_)
        | Req::SetLogHashRate(_) => {
            return Err(anyhow!("Mining RPC calls are not supported by Cuprate"))
        }
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L486-L499>
async fn get_height(
    mut state: CupratedRpcHandler,
    request: GetHeightRequest,
) -> Result<GetHeightResponse, Error> {
    let (height, hash) = helper::top_height(&mut state).await?;
    let hash = hex::encode(hash);

    Ok(GetHeightResponse {
        base: ResponseBase::OK,
        height,
        hash,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L979-L1227>
async fn get_transactions(
    state: CupratedRpcHandler,
    request: GetTransactionsRequest,
) -> Result<GetTransactionsResponse, Error> {
    Ok(GetTransactionsResponse {
        base: AccessResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L790-L815>
async fn get_alt_blocks_hashes(
    state: CupratedRpcHandler,
    request: GetAltBlocksHashesRequest,
) -> Result<GetAltBlocksHashesResponse, Error> {
    Ok(GetAltBlocksHashesResponse {
        base: AccessResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1229-L1305>
async fn is_key_image_spent(
    mut state: CupratedRpcHandler,
    request: IsKeyImageSpentRequest,
) -> Result<IsKeyImageSpentResponse, Error> {
    if state.is_restricted() && request.key_images.len() > RESTRICTED_SPENT_KEY_IMAGES_COUNT {
        return Err(anyhow!("Too many key images queried in restricted mode"));
    }

    let mut spent_status = Vec::with_capacity(request.key_images.len());

    for hex in request.key_images {
        let key_image = helper::hex_to_hash(hex)?;
        let status = helper::key_image_spent(&mut state, key_image).await?;
        spent_status.push(status.to_u8());
    }

    Ok(IsKeyImageSpentResponse {
        base: AccessResponseBase::OK,
        spent_status,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1307-L1411>
async fn send_raw_transaction(
    state: CupratedRpcHandler,
    request: SendRawTransactionRequest,
) -> Result<SendRawTransactionResponse, Error> {
    Ok(SendRawTransactionResponse {
        base: AccessResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1413-L1462>
async fn start_mining(
    state: CupratedRpcHandler,
    request: StartMiningRequest,
) -> Result<StartMiningResponse, Error> {
    unreachable!();
    Ok(StartMiningResponse {
        base: ResponseBase::OK,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1464-L1482>
async fn stop_mining(
    state: CupratedRpcHandler,
    request: StopMiningRequest,
) -> Result<StopMiningResponse, Error> {
    unreachable!();
    Ok(StopMiningResponse {
        base: ResponseBase::OK,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1484-L1523>
async fn mining_status(
    state: CupratedRpcHandler,
    request: MiningStatusRequest,
) -> Result<MiningStatusResponse, Error> {
    unreachable!();
    Ok(MiningStatusResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1525-L1535>
async fn save_bc(
    state: CupratedRpcHandler,
    request: SaveBcRequest,
) -> Result<SaveBcResponse, Error> {
    todo!();
    Ok(SaveBcResponse {
        base: ResponseBase::OK,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1537-L1582>
async fn get_peer_list(
    state: CupratedRpcHandler,
    request: GetPeerListRequest,
) -> Result<GetPeerListResponse, Error> {
    Ok(GetPeerListResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1626-L1639>
async fn set_log_hash_rate(
    state: CupratedRpcHandler,
    request: SetLogHashRateRequest,
) -> Result<SetLogHashRateResponse, Error> {
    unreachable!();
    Ok(SetLogHashRateResponse {
        base: ResponseBase::OK,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1641-L1652>
async fn set_log_level(
    state: CupratedRpcHandler,
    request: SetLogLevelRequest,
) -> Result<SetLogLevelResponse, Error> {
    todo!();
    Ok(SetLogLevelResponse {
        base: ResponseBase::OK,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1654-L1661>
async fn set_log_categories(
    state: CupratedRpcHandler,
    request: SetLogCategoriesRequest,
) -> Result<SetLogCategoriesResponse, Error> {
    Ok(SetLogCategoriesResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1758-L1778>
async fn set_bootstrap_daemon(
    state: CupratedRpcHandler,
    request: SetBootstrapDaemonRequest,
) -> Result<SetBootstrapDaemonResponse, Error> {
    todo!();
    Ok(SetBootstrapDaemonResponse { status: Status::Ok })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1663-L1687>
async fn get_transaction_pool(
    state: CupratedRpcHandler,
    request: GetTransactionPoolRequest,
) -> Result<GetTransactionPoolResponse, Error> {
    Ok(GetTransactionPoolResponse {
        base: AccessResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1741-L1756>
async fn get_transaction_pool_stats(
    state: CupratedRpcHandler,
    request: GetTransactionPoolStatsRequest,
) -> Result<GetTransactionPoolStatsResponse, Error> {
    Ok(GetTransactionPoolStatsResponse {
        base: AccessResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1780-L1788>
async fn stop_daemon(
    state: CupratedRpcHandler,
    request: StopDaemonRequest,
) -> Result<StopDaemonResponse, Error> {
    todo!();
    Ok(StopDaemonResponse { status: Status::Ok })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3066-L3077>
async fn get_limit(
    state: CupratedRpcHandler,
    request: GetLimitRequest,
) -> Result<GetLimitResponse, Error> {
    Ok(GetLimitResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3079-L3117>
async fn set_limit(
    state: CupratedRpcHandler,
    request: SetLimitRequest,
) -> Result<SetLimitResponse, Error> {
    Ok(SetLimitResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3119-L3127>
async fn out_peers(
    state: CupratedRpcHandler,
    request: OutPeersRequest,
) -> Result<OutPeersResponse, Error> {
    Ok(OutPeersResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3129-L3137>
async fn in_peers(
    state: CupratedRpcHandler,
    request: InPeersRequest,
) -> Result<InPeersResponse, Error> {
    Ok(InPeersResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L584-L599>
async fn get_net_stats(
    state: CupratedRpcHandler,
    request: GetNetStatsRequest,
) -> Result<GetNetStatsResponse, Error> {
    Ok(GetNetStatsResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L912-L957>
async fn get_outs(
    mut state: CupratedRpcHandler,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, Error> {
    if state.is_restricted() && request.outputs.len() > MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT {
        return Err(anyhow!("Too many outs requested"));
    }

    let mut outputs = HashMap::<u64, HashSet<u64>>::with_capacity(request.outputs.len());
    for out in request.outputs {
        outputs
            .entry(out.amount)
            .and_modify(|set| {
                set.insert(out.index);
            })
            .or_insert_with(|| HashSet::from([out.index]));
    }

    let outs = blockchain::outputs(&mut state.blockchain_read, outputs)
        .await?
        .into_iter()
        .flat_map(|(amount, index_map)| {
            index_map.into_iter().map(|(index, out)| OutKey {
                key: todo!(),
                mask: todo!(),
                unlocked: todo!(),
                height: usize_to_u64(out.height),
                txid: todo!(),
            })
        })
        .collect::<Vec<OutKey>>();

    // TODO: check txpool

    Ok(GetOutsResponse {
        base: ResponseBase::OK,
        outs,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3139-L3240>
async fn update(
    state: CupratedRpcHandler,
    request: UpdateRequest,
) -> Result<UpdateResponse, Error> {
    Ok(UpdateResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3242-L3252>
async fn pop_blocks(
    mut state: CupratedRpcHandler,
    request: PopBlocksRequest,
) -> Result<PopBlocksResponse, Error> {
    let height =
        blockchain_manager::pop_blocks(&mut state.blockchain_manager, request.nblocks).await?;

    Ok(PopBlocksResponse {
        base: ResponseBase::OK,
        height,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1713-L1739>
async fn get_transaction_pool_hashes(
    state: CupratedRpcHandler,
    request: GetTransactionPoolHashesRequest,
) -> Result<GetTransactionPoolHashesResponse, Error> {
    Ok(GetTransactionPoolHashesResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L193-L225>
async fn get_public_nodes(
    state: CupratedRpcHandler,
    request: GetPublicNodesRequest,
) -> Result<GetPublicNodesResponse, Error> {
    Ok(GetPublicNodesResponse {
        base: ResponseBase::OK,
        ..todo!()
    })
}
