use anyhow::Error;

use cuprate_rpc_types::other::{
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
    SetLogCategoriesResponse, SetLogHashRateRequest, SetLogHashRateResponse, SetLogLevelRequest,
    SetLogLevelResponse, StartMiningRequest, StartMiningResponse, StopDaemonRequest,
    StopDaemonResponse, StopMiningRequest, StopMiningResponse, UpdateRequest, UpdateResponse,
};

use crate::rpc::CupratedRpcHandler;

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

async fn get_height(
    state: CupratedRpcHandler,
    request: GetHeightRequest,
) -> Result<GetHeightResponse, Error> {
    todo!()
}

async fn get_transactions(
    state: CupratedRpcHandler,
    request: GetTransactionsRequest,
) -> Result<GetTransactionsResponse, Error> {
    todo!()
}

async fn get_alt_blocks_hashes(
    state: CupratedRpcHandler,
    request: GetAltBlocksHashesRequest,
) -> Result<GetAltBlocksHashesResponse, Error> {
    todo!()
}

async fn is_key_image_spent(
    state: CupratedRpcHandler,
    request: IsKeyImageSpentRequest,
) -> Result<IsKeyImageSpentResponse, Error> {
    todo!()
}

async fn send_raw_transaction(
    state: CupratedRpcHandler,
    request: SendRawTransactionRequest,
) -> Result<SendRawTransactionResponse, Error> {
    todo!()
}

async fn start_mining(
    state: CupratedRpcHandler,
    request: StartMiningRequest,
) -> Result<StartMiningResponse, Error> {
    todo!()
}

async fn stop_mining(
    state: CupratedRpcHandler,
    request: StopMiningRequest,
) -> Result<StopMiningResponse, Error> {
    todo!()
}

async fn mining_status(
    state: CupratedRpcHandler,
    request: MiningStatusRequest,
) -> Result<MiningStatusResponse, Error> {
    todo!()
}

async fn save_bc(
    state: CupratedRpcHandler,
    request: SaveBcRequest,
) -> Result<SaveBcResponse, Error> {
    todo!()
}

async fn get_peer_list(
    state: CupratedRpcHandler,
    request: GetPeerListRequest,
) -> Result<GetPeerListResponse, Error> {
    todo!()
}

async fn set_log_hash_rate(
    state: CupratedRpcHandler,
    request: SetLogHashRateRequest,
) -> Result<SetLogHashRateResponse, Error> {
    todo!()
}

async fn set_log_level(
    state: CupratedRpcHandler,
    request: SetLogLevelRequest,
) -> Result<SetLogLevelResponse, Error> {
    todo!()
}

async fn set_log_categories(
    state: CupratedRpcHandler,
    request: SetLogCategoriesRequest,
) -> Result<SetLogCategoriesResponse, Error> {
    todo!()
}

async fn set_bootstrap_daemon(
    state: CupratedRpcHandler,
    request: SetBootstrapDaemonRequest,
) -> Result<SetBootstrapDaemonResponse, Error> {
    todo!()
}

async fn get_transaction_pool(
    state: CupratedRpcHandler,
    request: GetTransactionPoolRequest,
) -> Result<GetTransactionPoolResponse, Error> {
    todo!()
}

async fn get_transaction_pool_stats(
    state: CupratedRpcHandler,
    request: GetTransactionPoolStatsRequest,
) -> Result<GetTransactionPoolStatsResponse, Error> {
    todo!()
}

async fn stop_daemon(
    state: CupratedRpcHandler,
    request: StopDaemonRequest,
) -> Result<StopDaemonResponse, Error> {
    todo!()
}

async fn get_limit(
    state: CupratedRpcHandler,
    request: GetLimitRequest,
) -> Result<GetLimitResponse, Error> {
    todo!()
}

async fn set_limit(
    state: CupratedRpcHandler,
    request: SetLimitRequest,
) -> Result<SetLimitResponse, Error> {
    todo!()
}

async fn out_peers(
    state: CupratedRpcHandler,
    request: OutPeersRequest,
) -> Result<OutPeersResponse, Error> {
    todo!()
}

async fn in_peers(
    state: CupratedRpcHandler,
    request: InPeersRequest,
) -> Result<InPeersResponse, Error> {
    todo!()
}

async fn get_net_stats(
    state: CupratedRpcHandler,
    request: GetNetStatsRequest,
) -> Result<GetNetStatsResponse, Error> {
    todo!()
}

async fn get_outs(
    state: CupratedRpcHandler,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, Error> {
    todo!()
}

async fn update(
    state: CupratedRpcHandler,
    request: UpdateRequest,
) -> Result<UpdateResponse, Error> {
    todo!()
}

async fn pop_blocks(
    state: CupratedRpcHandler,
    request: PopBlocksRequest,
) -> Result<PopBlocksResponse, Error> {
    todo!()
}

async fn get_transaction_pool_hashes(
    state: CupratedRpcHandler,
    request: GetTransactionPoolHashesRequest,
) -> Result<GetTransactionPoolHashesResponse, Error> {
    todo!()
}

async fn get_public_nodes(
    state: CupratedRpcHandler,
    request: GetPublicNodesRequest,
) -> Result<GetPublicNodesResponse, Error> {
    todo!()
}
