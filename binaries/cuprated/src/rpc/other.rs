use cuprate_rpc_types::other::{
    GetAltBlocksHashesRequest, GetAltBlocksHashesResponse, GetHeightRequest, GetHeightResponse,
    GetLimitRequest, GetLimitResponse, GetNetStatsRequest, GetNetStatsResponse, GetOutsRequest,
    GetOutsResponse, GetPeerListRequest, GetPeerListResponse, GetPublicNodesRequest,
    GetPublicNodesResponse, GetTransactionPoolHashesRequest, GetTransactionPoolHashesResponse,
    GetTransactionPoolRequest, GetTransactionPoolResponse, GetTransactionPoolStatsRequest,
    GetTransactionPoolStatsResponse, GetTransactionsRequest, GetTransactionsResponse,
    InPeersRequest, InPeersResponse, IsKeyImageSpentRequest, IsKeyImageSpentResponse,
    MiningStatusRequest, MiningStatusResponse, OutPeersRequest, OutPeersResponse, PopBlocksRequest,
    PopBlocksResponse, SaveBcRequest, SaveBcResponse, SendRawTransactionRequest,
    SendRawTransactionResponse, SetBootstrapDaemonRequest, SetBootstrapDaemonResponse,
    SetLimitRequest, SetLimitResponse, SetLogCategoriesRequest, SetLogCategoriesResponse,
    SetLogHashRateRequest, SetLogHashRateResponse, SetLogLevelRequest, SetLogLevelResponse,
    StartMiningRequest, StartMiningResponse, StopDaemonRequest, StopDaemonResponse,
    StopMiningRequest, StopMiningResponse, UpdateRequest, UpdateResponse,
};

use crate::rpc::CupratedRpcHandler;

pub(super) async fn map_request(
    state: CupratedRpcHandler,
    request: OtherRpcRequest,
) -> OtherRpcResponse {
    use OtherRpcRequest as Req;
    use OtherRpcResponse as Resp;

    match request {
        Req::GetHeight(r) => Resp::GetHeight(get_height(state, r)),
        Req::GetTransactions(r) => Resp::GetTransactions(get_transactions(state, r)),
        Req::GetAltBlocksHashes(r) => Resp::GetAltBlocksHashes(get_alt_blocks_hashes(state, r)),
        Req::IsKeyImageSpent(r) => Resp::IsKeyImageSpent(is_key_image_spent(state, r)),
        Req::SendRawTransaction(r) => Resp::SendRawTransaction(send_raw_transaction(state, r)),
        Req::StartMining(r) => Resp::StartMining(start_mining(state, r)),
        Req::StopMining(r) => Resp::StopMining(stop_mining(state, r)),
        Req::MiningStatus(r) => Resp::MiningStatus(mining_status(state, r)),
        Req::SaveBc(r) => Resp::SaveBc(save_bc(state, r)),
        Req::GetPeerList(r) => Resp::GetPeerList(get_peer_list(state, r)),
        Req::SetLogHashRate(r) => Resp::SetLogHashRate(set_log_hash_rate(state, r)),
        Req::SetLogLevel(r) => Resp::SetLogLevel(set_log_level(state, r)),
        Req::SetLogCategories(r) => Resp::SetLogCategories(set_log_categories(state, r)),
        Req::SetBootstrapDaemon(r) => Resp::SetBootstrapDaemon(set_bootstrap_daemon(state, r)),
        Req::GetTransactionPool(r) => Resp::GetTransactionPool(get_transaction_pool(state, r)),
        Req::GetTransactionPoolStats(r) => {
            Resp::GetTransactionPoolStats(get_transaction_pool_stats(state, r))
        }
        Req::StopDaemon(r) => Resp::StopDaemon(stop_daemon(state, r)),
        Req::GetLimit(r) => Resp::GetLimit(get_limit(state, r)),
        Req::SetLimit(r) => Resp::SetLimit(set_limit(state, r)),
        Req::OutPeers(r) => Resp::OutPeers(out_peers(state, r)),
        Req::InPeers(r) => Resp::InPeers(in_peers(state, r)),
        Req::GetNetStats(r) => Resp::GetNetStats(get_net_stats(state, r)),
        Req::GetOuts(r) => Resp::GetOuts(get_outs(state, r)),
        Req::Update(r) => Resp::Update(update(state, r)),
        Req::PopBlocks(r) => Resp::PopBlocks(pop_blocks(state, r)),
        Req::GetTransactionPoolHashes(r) => {
            Resp::GetTransactionPoolHashes(get_transaction_pool_hashes(state, r))
        }
        Req::GetPublicNodes(r) => Resp::GetPublicNodes(get_public_nodes(state, r)),
    }
}

async fn get_height(state: CupratedRpcHandler, request: GetHeightRequest) -> GetHeightResponse {
    todo!()
}

async fn get_transactions(
    state: CupratedRpcHandler,
    request: GetTransactionsRequest,
) -> GetTransactionsResponse {
    todo!()
}

async fn get_alt_blocks_hashes(
    state: CupratedRpcHandler,
    request: GetAltBlocksHashesRequest,
) -> GetAltBlocksHashesResponse {
    todo!()
}

async fn is_key_image_spent(
    state: CupratedRpcHandler,
    request: IsKeyImageSpentRequest,
) -> IsKeyImageSpentResponse {
    todo!()
}

async fn send_raw_transaction(
    state: CupratedRpcHandler,
    request: SendRawTransactionRequest,
) -> SendRawTransactionResponse {
    todo!()
}

async fn start_mining(
    state: CupratedRpcHandler,
    request: StartMiningRequest,
) -> StartMiningResponse {
    todo!()
}

async fn stop_mining(state: CupratedRpcHandler, request: StopMiningRequest) -> StopMiningResponse {
    todo!()
}

async fn mining_status(
    state: CupratedRpcHandler,
    request: MiningStatusRequest,
) -> MiningStatusResponse {
    todo!()
}

async fn save_bc(state: CupratedRpcHandler, request: SaveBcRequest) -> SaveBcResponse {
    todo!()
}

async fn get_peer_list(
    state: CupratedRpcHandler,
    request: GetPeerListRequest,
) -> GetPeerListResponse {
    todo!()
}

async fn set_log_hash_rate(
    state: CupratedRpcHandler,
    request: SetLogHashRateRequest,
) -> SetLogHashRateResponse {
    todo!()
}

async fn set_log_level(
    state: CupratedRpcHandler,
    request: SetLogLevelRequest,
) -> SetLogLevelResponse {
    todo!()
}

async fn set_log_categories(
    state: CupratedRpcHandler,
    request: SetLogCategoriesRequest,
) -> SetLogCategoriesResponse {
    todo!()
}

async fn set_bootstrap_daemon(
    state: CupratedRpcHandler,
    request: SetBootstrapDaemonRequest,
) -> SetBootstrapDaemonResponse {
    todo!()
}

async fn get_transaction_pool(
    state: CupratedRpcHandler,
    request: GetTransactionPoolRequest,
) -> GetTransactionPoolResponse {
    todo!()
}

async fn get_transaction_pool_stats(
    state: CupratedRpcHandler,
    request: GetTransactionPoolStatsRequest,
) -> GetTransactionPoolStatsResponse {
    todo!()
}

async fn stop_daemon(state: CupratedRpcHandler, request: StopDaemonRequest) -> StopDaemonResponse {
    todo!()
}

async fn get_limit(state: CupratedRpcHandler, request: GetLimitRequest) -> GetLimitResponse {
    todo!()
}

async fn set_limit(state: CupratedRpcHandler, request: SetLimitRequest) -> SetLimitResponse {
    todo!()
}

async fn out_peers(state: CupratedRpcHandler, request: OutPeersRequest) -> OutPeersResponse {
    todo!()
}

async fn in_peers(state: CupratedRpcHandler, request: InPeersRequest) -> InPeersResponse {
    todo!()
}

async fn get_net_stats(
    state: CupratedRpcHandler,
    request: GetNetStatsRequest,
) -> GetNetStatsResponse {
    todo!()
}

async fn get_outs(state: CupratedRpcHandler, request: GetOutsRequest) -> GetOutsResponse {
    todo!()
}

async fn update(state: CupratedRpcHandler, request: UpdateRequest) -> UpdateResponse {
    todo!()
}

async fn pop_blocks(state: CupratedRpcHandler, request: PopBlocksRequest) -> PopBlocksResponse {
    todo!()
}

async fn get_transaction_pool_hashes(
    state: CupratedRpcHandler,
    request: GetTransactionPoolHashesRequest,
) -> GetTransactionPoolHashesResponse {
    todo!()
}

async fn get_public_nodes(
    state: CupratedRpcHandler,
    request: GetPublicNodesRequest,
) -> GetPublicNodesResponse {
    todo!()
}
