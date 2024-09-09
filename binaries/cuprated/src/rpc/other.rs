use anyhow::{anyhow, Error};

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

use crate::{
    rpc::CupratedRpcHandlerState,
    rpc::{blockchain, helper},
};

use super::RESTRICTED_SPENT_KEY_IMAGES_COUNT;

/// Map a [`OtherRequest`] to the function that will lead to a [`OtherResponse`].
pub(super) async fn map_request(
    state: CupratedRpcHandlerState,
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
        Req::StartMining(r) => Resp::StartMining(start_mining(state, r).await?),
        Req::StopMining(r) => Resp::StopMining(stop_mining(state, r).await?),
        Req::MiningStatus(r) => Resp::MiningStatus(mining_status(state, r).await?),
        Req::SaveBc(r) => Resp::SaveBc(save_bc(state, r).await?),
        Req::GetPeerList(r) => Resp::GetPeerList(get_peer_list(state, r).await?),
        Req::SetLogHashRate(r) => Resp::SetLogHashRate(set_log_hash_rate(state, r).await?),
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
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L486-L499>
async fn get_height(
    mut state: CupratedRpcHandlerState,
    request: GetHeightRequest,
) -> Result<GetHeightResponse, Error> {
    let (height, hash) = helper::top_height(&mut state).await?;
    let hash = hex::encode(hash);

    Ok(GetHeightResponse {
        base: ResponseBase::ok(),
        height,
        hash,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L979-L1227>
async fn get_transactions(
    state: CupratedRpcHandlerState,
    request: GetTransactionsRequest,
) -> Result<GetTransactionsResponse, Error> {
    Ok(GetTransactionsResponse {
        base: AccessResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L790-L815>
async fn get_alt_blocks_hashes(
    state: CupratedRpcHandlerState,
    request: GetAltBlocksHashesRequest,
) -> Result<GetAltBlocksHashesResponse, Error> {
    Ok(GetAltBlocksHashesResponse {
        base: AccessResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1229-L1305>
async fn is_key_image_spent(
    mut state: CupratedRpcHandlerState,
    request: IsKeyImageSpentRequest,
) -> Result<IsKeyImageSpentResponse, Error> {
    if state.restricted && request.key_images.len() > RESTRICTED_SPENT_KEY_IMAGES_COUNT {
        return Err(anyhow!("Too many key images queried in restricted mode"));
    }

    let mut spent_status = Vec::with_capacity(request.key_images.len());

    for hex in request.key_images {
        let key_image = helper::hex_to_hash(hex)?;
        let status = helper::key_image_spent(&mut state, key_image).await?;
        spent_status.push(status.to_u8());
    }

    Ok(IsKeyImageSpentResponse {
        base: AccessResponseBase::ok(),
        spent_status,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1307-L1411>
async fn send_raw_transaction(
    state: CupratedRpcHandlerState,
    request: SendRawTransactionRequest,
) -> Result<SendRawTransactionResponse, Error> {
    Ok(SendRawTransactionResponse {
        base: AccessResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1413-L1462>
async fn start_mining(
    state: CupratedRpcHandlerState,
    request: StartMiningRequest,
) -> Result<StartMiningResponse, Error> {
    todo!();
    Ok(StartMiningResponse {
        base: ResponseBase::ok(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1464-L1482>
async fn stop_mining(
    state: CupratedRpcHandlerState,
    request: StopMiningRequest,
) -> Result<StopMiningResponse, Error> {
    todo!();
    Ok(StopMiningResponse {
        base: ResponseBase::ok(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1484-L1523>
async fn mining_status(
    state: CupratedRpcHandlerState,
    request: MiningStatusRequest,
) -> Result<MiningStatusResponse, Error> {
    Ok(MiningStatusResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1525-L1535>
async fn save_bc(
    state: CupratedRpcHandlerState,
    request: SaveBcRequest,
) -> Result<SaveBcResponse, Error> {
    todo!();
    Ok(SaveBcResponse {
        base: ResponseBase::ok(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1537-L1582>
async fn get_peer_list(
    state: CupratedRpcHandlerState,
    request: GetPeerListRequest,
) -> Result<GetPeerListResponse, Error> {
    Ok(GetPeerListResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1626-L1639>
async fn set_log_hash_rate(
    state: CupratedRpcHandlerState,
    request: SetLogHashRateRequest,
) -> Result<SetLogHashRateResponse, Error> {
    todo!();
    Ok(SetLogHashRateResponse {
        base: ResponseBase::ok(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1641-L1652>
async fn set_log_level(
    state: CupratedRpcHandlerState,
    request: SetLogLevelRequest,
) -> Result<SetLogLevelResponse, Error> {
    todo!();
    Ok(SetLogLevelResponse {
        base: ResponseBase::ok(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1654-L1661>
async fn set_log_categories(
    state: CupratedRpcHandlerState,
    request: SetLogCategoriesRequest,
) -> Result<SetLogCategoriesResponse, Error> {
    Ok(SetLogCategoriesResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1758-L1778>
async fn set_bootstrap_daemon(
    state: CupratedRpcHandlerState,
    request: SetBootstrapDaemonRequest,
) -> Result<SetBootstrapDaemonResponse, Error> {
    todo!();
    Ok(SetBootstrapDaemonResponse { status: Status::Ok })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1663-L1687>
async fn get_transaction_pool(
    state: CupratedRpcHandlerState,
    request: GetTransactionPoolRequest,
) -> Result<GetTransactionPoolResponse, Error> {
    Ok(GetTransactionPoolResponse {
        base: AccessResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1741-L1756>
async fn get_transaction_pool_stats(
    state: CupratedRpcHandlerState,
    request: GetTransactionPoolStatsRequest,
) -> Result<GetTransactionPoolStatsResponse, Error> {
    Ok(GetTransactionPoolStatsResponse {
        base: AccessResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1780-L1788>
async fn stop_daemon(
    state: CupratedRpcHandlerState,
    request: StopDaemonRequest,
) -> Result<StopDaemonResponse, Error> {
    todo!();
    Ok(StopDaemonResponse { status: Status::Ok })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3066-L3077>
async fn get_limit(
    state: CupratedRpcHandlerState,
    request: GetLimitRequest,
) -> Result<GetLimitResponse, Error> {
    Ok(GetLimitResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3079-L3117>
async fn set_limit(
    state: CupratedRpcHandlerState,
    request: SetLimitRequest,
) -> Result<SetLimitResponse, Error> {
    Ok(SetLimitResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3119-L3127>
async fn out_peers(
    state: CupratedRpcHandlerState,
    request: OutPeersRequest,
) -> Result<OutPeersResponse, Error> {
    Ok(OutPeersResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3129-L3137>
async fn in_peers(
    state: CupratedRpcHandlerState,
    request: InPeersRequest,
) -> Result<InPeersResponse, Error> {
    Ok(InPeersResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L584-L599>
async fn get_net_stats(
    state: CupratedRpcHandlerState,
    request: GetNetStatsRequest,
) -> Result<GetNetStatsResponse, Error> {
    Ok(GetNetStatsResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L912-L957>
async fn get_outs(
    state: CupratedRpcHandlerState,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, Error> {
    Ok(GetOutsResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3139-L3240>
async fn update(
    state: CupratedRpcHandlerState,
    request: UpdateRequest,
) -> Result<UpdateResponse, Error> {
    Ok(UpdateResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3242-L3252>
async fn pop_blocks(
    state: CupratedRpcHandlerState,
    request: PopBlocksRequest,
) -> Result<PopBlocksResponse, Error> {
    Ok(PopBlocksResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1713-L1739>
async fn get_transaction_pool_hashes(
    state: CupratedRpcHandlerState,
    request: GetTransactionPoolHashesRequest,
) -> Result<GetTransactionPoolHashesResponse, Error> {
    Ok(GetTransactionPoolHashesResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L193-L225>
async fn get_public_nodes(
    state: CupratedRpcHandlerState,
    request: GetPublicNodesRequest,
) -> Result<GetPublicNodesResponse, Error> {
    Ok(GetPublicNodesResponse {
        base: ResponseBase::ok(),
        ..todo!()
    })
}
