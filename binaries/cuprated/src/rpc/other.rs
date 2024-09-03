use cuprate_rpc_interface::RpcError;
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

pub(super) fn map_request(
    state: CupratedRpcHandler,
    request: OtherRequest,
) -> Result<OtherResponse, RpcError> {
    use OtherRequest as Req;
    use OtherResponse as Resp;

    Ok(match request {
        Req::GetHeight(r) => Resp::GetHeight(get_height(state, r)?),
        Req::GetTransactions(r) => Resp::GetTransactions(get_transactions(state, r)?),
        Req::GetAltBlocksHashes(r) => Resp::GetAltBlocksHashes(get_alt_blocks_hashes(state, r)?),
        Req::IsKeyImageSpent(r) => Resp::IsKeyImageSpent(is_key_image_spent(state, r)?),
        Req::SendRawTransaction(r) => Resp::SendRawTransaction(send_raw_transaction(state, r)?),
        Req::StartMining(r) => Resp::StartMining(start_mining(state, r)?),
        Req::StopMining(r) => Resp::StopMining(stop_mining(state, r)?),
        Req::MiningStatus(r) => Resp::MiningStatus(mining_status(state, r)?),
        Req::SaveBc(r) => Resp::SaveBc(save_bc(state, r)?),
        Req::GetPeerList(r) => Resp::GetPeerList(get_peer_list(state, r)?),
        Req::SetLogHashRate(r) => Resp::SetLogHashRate(set_log_hash_rate(state, r)?),
        Req::SetLogLevel(r) => Resp::SetLogLevel(set_log_level(state, r)?),
        Req::SetLogCategories(r) => Resp::SetLogCategories(set_log_categories(state, r)?),
        Req::SetBootstrapDaemon(r) => Resp::SetBootstrapDaemon(set_bootstrap_daemon(state, r)?),
        Req::GetTransactionPool(r) => Resp::GetTransactionPool(get_transaction_pool(state, r)?),
        Req::GetTransactionPoolStats(r) => {
            Resp::GetTransactionPoolStats(get_transaction_pool_stats(state, r)?)
        }
        Req::StopDaemon(r) => Resp::StopDaemon(stop_daemon(state, r)?),
        Req::GetLimit(r) => Resp::GetLimit(get_limit(state, r)?),
        Req::SetLimit(r) => Resp::SetLimit(set_limit(state, r)?),
        Req::OutPeers(r) => Resp::OutPeers(out_peers(state, r)?),
        Req::InPeers(r) => Resp::InPeers(in_peers(state, r)?),
        Req::GetNetStats(r) => Resp::GetNetStats(get_net_stats(state, r)?),
        Req::GetOuts(r) => Resp::GetOuts(get_outs(state, r)?),
        Req::Update(r) => Resp::Update(update(state, r)?),
        Req::PopBlocks(r) => Resp::PopBlocks(pop_blocks(state, r)?),
        Req::GetTransactionPoolHashes(r) => {
            Resp::GetTransactionPoolHashes(get_transaction_pool_hashes(state, r)?)
        }
        Req::GetPublicNodes(r) => Resp::GetPublicNodes(get_public_nodes(state, r)?),
    })
}

fn get_height(
    state: CupratedRpcHandler,
    request: GetHeightRequest,
) -> Result<GetHeightResponse, RpcError> {
    todo!()
}

fn get_transactions(
    state: CupratedRpcHandler,
    request: GetTransactionsRequest,
) -> Result<GetTransactionsResponse, RpcError> {
    todo!()
}

fn get_alt_blocks_hashes(
    state: CupratedRpcHandler,
    request: GetAltBlocksHashesRequest,
) -> Result<GetAltBlocksHashesResponse, RpcError> {
    todo!()
}

fn is_key_image_spent(
    state: CupratedRpcHandler,
    request: IsKeyImageSpentRequest,
) -> Result<IsKeyImageSpentResponse, RpcError> {
    todo!()
}

fn send_raw_transaction(
    state: CupratedRpcHandler,
    request: SendRawTransactionRequest,
) -> Result<SendRawTransactionResponse, RpcError> {
    todo!()
}

fn start_mining(
    state: CupratedRpcHandler,
    request: StartMiningRequest,
) -> Result<StartMiningResponse, RpcError> {
    todo!()
}

fn stop_mining(
    state: CupratedRpcHandler,
    request: StopMiningRequest,
) -> Result<StopMiningResponse, RpcError> {
    todo!()
}

fn mining_status(
    state: CupratedRpcHandler,
    request: MiningStatusRequest,
) -> Result<MiningStatusResponse, RpcError> {
    todo!()
}

fn save_bc(state: CupratedRpcHandler, request: SaveBcRequest) -> Result<SaveBcResponse, RpcError> {
    todo!()
}

fn get_peer_list(
    state: CupratedRpcHandler,
    request: GetPeerListRequest,
) -> Result<GetPeerListResponse, RpcError> {
    todo!()
}

fn set_log_hash_rate(
    state: CupratedRpcHandler,
    request: SetLogHashRateRequest,
) -> Result<SetLogHashRateResponse, RpcError> {
    todo!()
}

fn set_log_level(
    state: CupratedRpcHandler,
    request: SetLogLevelRequest,
) -> Result<SetLogLevelResponse, RpcError> {
    todo!()
}

fn set_log_categories(
    state: CupratedRpcHandler,
    request: SetLogCategoriesRequest,
) -> Result<SetLogCategoriesResponse, RpcError> {
    todo!()
}

fn set_bootstrap_daemon(
    state: CupratedRpcHandler,
    request: SetBootstrapDaemonRequest,
) -> Result<SetBootstrapDaemonResponse, RpcError> {
    todo!()
}

fn get_transaction_pool(
    state: CupratedRpcHandler,
    request: GetTransactionPoolRequest,
) -> Result<GetTransactionPoolResponse, RpcError> {
    todo!()
}

fn get_transaction_pool_stats(
    state: CupratedRpcHandler,
    request: GetTransactionPoolStatsRequest,
) -> Result<GetTransactionPoolStatsResponse, RpcError> {
    todo!()
}

fn stop_daemon(
    state: CupratedRpcHandler,
    request: StopDaemonRequest,
) -> Result<StopDaemonResponse, RpcError> {
    todo!()
}

fn get_limit(
    state: CupratedRpcHandler,
    request: GetLimitRequest,
) -> Result<GetLimitResponse, RpcError> {
    todo!()
}

fn set_limit(
    state: CupratedRpcHandler,
    request: SetLimitRequest,
) -> Result<SetLimitResponse, RpcError> {
    todo!()
}

fn out_peers(
    state: CupratedRpcHandler,
    request: OutPeersRequest,
) -> Result<OutPeersResponse, RpcError> {
    todo!()
}

fn in_peers(
    state: CupratedRpcHandler,
    request: InPeersRequest,
) -> Result<InPeersResponse, RpcError> {
    todo!()
}

fn get_net_stats(
    state: CupratedRpcHandler,
    request: GetNetStatsRequest,
) -> Result<GetNetStatsResponse, RpcError> {
    todo!()
}

fn get_outs(
    state: CupratedRpcHandler,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, RpcError> {
    todo!()
}

fn update(state: CupratedRpcHandler, request: UpdateRequest) -> Result<UpdateResponse, RpcError> {
    todo!()
}

fn pop_blocks(
    state: CupratedRpcHandler,
    request: PopBlocksRequest,
) -> Result<PopBlocksResponse, RpcError> {
    todo!()
}

fn get_transaction_pool_hashes(
    state: CupratedRpcHandler,
    request: GetTransactionPoolHashesRequest,
) -> Result<GetTransactionPoolHashesResponse, RpcError> {
    todo!()
}

fn get_public_nodes(
    state: CupratedRpcHandler,
    request: GetPublicNodesRequest,
) -> Result<GetPublicNodesResponse, RpcError> {
    todo!()
}
