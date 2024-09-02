use std::sync::Arc;

use cuprate_rpc_types::json::{
    AddAuxPowRequest, AddAuxPowResponse, BannedRequest, BannedResponse, CalcPowRequest,
    CalcPowResponse, FlushCacheRequest, FlushCacheResponse, FlushTransactionPoolRequest,
    FlushTransactionPoolResponse, GenerateBlocksRequest, GenerateBlocksResponse,
    GetAlternateChainsRequest, GetAlternateChainsResponse, GetBansRequest, GetBansResponse,
    GetBlockCountRequest, GetBlockCountResponse, GetBlockHeaderByHashRequest,
    GetBlockHeaderByHashResponse, GetBlockHeaderByHeightRequest, GetBlockHeaderByHeightResponse,
    GetBlockHeadersRangeRequest, GetBlockHeadersRangeResponse, GetBlockRequest, GetBlockResponse,
    GetCoinbaseTxSumRequest, GetCoinbaseTxSumResponse, GetConnectionsRequest,
    GetConnectionsResponse, GetFeeEstimateRequest, GetFeeEstimateResponse, GetInfoRequest,
    GetInfoResponse, GetLastBlockHeaderRequest, GetLastBlockHeaderResponse, GetMinerDataRequest,
    GetMinerDataResponse, GetOutputHistogramRequest, GetOutputHistogramResponse,
    GetTransactionPoolBacklogRequest, GetTransactionPoolBacklogResponse, GetTxIdsLooseRequest,
    GetTxIdsLooseResponse, GetVersionRequest, GetVersionResponse, HardForkInfoRequest,
    HardForkInfoResponse, JsonRpcRequest, JsonRpcResponse, OnGetBlockHashRequest,
    OnGetBlockHashResponse, PruneBlockchainRequest, PruneBlockchainResponse, RelayTxRequest,
    RelayTxResponse, SetBansRequest, SetBansResponse, SubmitBlockRequest, SubmitBlockResponse,
    SyncInfoRequest, SyncInfoResponse,
};

use crate::rpc::CupratedRpcHandler;

async fn map_request(state: CupratedRpcHandler, request: JsonRpcRequest) -> JsonRpcResponse {
    use JsonRpcRequest as Req;
    use JsonRpcResponse as Resp;

    match request {
        Req::GetBlockCount(r) => Resp::GetBlockCount(get_block_count(state, r)),
        Req::OnGetBlockHash(r) => Resp::OnGetBlockHash(on_get_block_hash(state, r)),
        Req::SubmitBlock(r) => Resp::SubmitBlock(submit_block(state, r)),
        Req::GenerateBlocks(r) => Resp::GenerateBlocks(generate_blocks(state, r)),
        Req::GetLastBlockHeader(r) => Resp::GetLastBlockHeader(get_last_block_header(state, r)),
        Req::GetBlockHeaderByHash(r) => {
            Resp::GetBlockHeaderByHash(get_block_header_by_hash(state, r))
        }
        Req::GetBlockHeaderByHeight(r) => {
            Resp::GetBlockHeaderByHeight(get_block_header_by_height(state, r))
        }
        Req::GetBlockHeadersRange(r) => {
            Resp::GetBlockHeadersRange(get_block_headers_range(state, r))
        }
        Req::GetBlock(r) => Resp::GetBlock(get_block(state, r)),
        Req::GetConnections(r) => Resp::GetConnections(get_connections(state, r)),
        Req::GetInfo(r) => Resp::GetInfo(get_info(state, r)),
        Req::HardForkInfo(r) => Resp::HardForkInfo(hard_fork_info(state, r)),
        Req::SetBans(r) => Resp::SetBans(set_bans(state, r)),
        Req::GetBans(r) => Resp::GetBans(get_bans(state, r)),
        Req::Banned(r) => Resp::Banned(banned(state, r)),
        Req::FlushTransactionPool(r) => {
            Resp::FlushTransactionPool(flush_transaction_pool(state, r))
        }
        Req::GetOutputHistogram(r) => Resp::GetOutputHistogram(get_output_histogram(state, r)),
        Req::GetCoinbaseTxSum(r) => Resp::GetCoinbaseTxSum(get_coinbase_tx_sum(state, r)),
        Req::GetVersion(r) => Resp::GetVersion(get_version(state, r)),
        Req::GetFeeEstimate(r) => Resp::GetFeeEstimate(get_fee_estimate(state, r)),
        Req::GetAlternateChains(r) => Resp::GetAlternateChains(get_alternate_chains(state, r)),
        Req::RelayTx(r) => Resp::RelayTx(relay_tx(state, r)),
        Req::SyncInfo(r) => Resp::SyncInfo(sync_info(state, r)),
        Req::GetTransactionPoolBacklog(r) => {
            Resp::GetTransactionPoolBacklog(get_transaction_pool_backlog(state, r))
        }
        Req::GetMinerData(r) => Resp::GetMinerData(get_miner_data(state, r)),
        Req::PruneBlockchain(r) => Resp::PruneBlockchain(prune_blockchain(state, r)),
        Req::CalcPow(r) => Resp::CalcPow(calc_pow(state, r)),
        Req::FlushCache(r) => Resp::FlushCache(flush_cache(state, r)),
        Req::AddAuxPow(r) => Resp::AddAuxPow(add_aux_pow(state, r)),
        Req::GetTxIdsLoose(r) => Resp::GetTxIdsLoose(get_tx_ids_loose(state, r)),
    }
}

async fn get_block_count(
    state: CupratedRpcHandler,
    request: GetBlockCountRequest,
) -> GetBlockCountResponse {
    todo!()
}

async fn on_get_block_hash(
    state: CupratedRpcHandler,
    request: OnGetBlockHashRequest,
) -> OnGetBlockHashResponse {
    todo!()
}

async fn submit_block(
    state: CupratedRpcHandler,
    request: SubmitBlockRequest,
) -> SubmitBlockResponse {
    todo!()
}

async fn generate_blocks(
    state: CupratedRpcHandler,
    request: GenerateBlocksRequest,
) -> GenerateBlocksResponse {
    todo!()
}

async fn get_last_block_header(
    state: CupratedRpcHandler,
    request: GetLastBlockHeaderRequest,
) -> GetLastBlockHeaderResponse {
    todo!()
}

async fn get_block_header_by_hash(
    state: CupratedRpcHandler,
    request: GetBlockHeaderByHashRequest,
) -> GetBlockHeaderByHashResponse {
    todo!()
}

async fn get_block_header_by_height(
    state: CupratedRpcHandler,
    request: GetBlockHeaderByHeightRequest,
) -> GetBlockHeaderByHeightResponse {
    todo!()
}

async fn get_block_headers_range(
    state: CupratedRpcHandler,
    request: GetBlockHeadersRangeRequest,
) -> GetBlockHeadersRangeResponse {
    todo!()
}

async fn get_block(state: CupratedRpcHandler, request: GetBlockRequest) -> GetBlockResponse {
    todo!()
}

async fn get_connections(
    state: CupratedRpcHandler,
    request: GetConnectionsRequest,
) -> GetConnectionsResponse {
    todo!()
}

async fn get_info(state: CupratedRpcHandler, request: GetInfoRequest) -> GetInfoResponse {
    todo!()
}

async fn hard_fork_info(
    state: CupratedRpcHandler,
    request: HardForkInfoRequest,
) -> HardForkInfoResponse {
    todo!()
}

async fn set_bans(state: CupratedRpcHandler, request: SetBansRequest) -> SetBansResponse {
    todo!()
}

async fn get_bans(state: CupratedRpcHandler, request: GetBansRequest) -> GetBansResponse {
    todo!()
}

async fn banned(state: CupratedRpcHandler, request: BannedRequest) -> BannedResponse {
    todo!()
}

async fn flush_transaction_pool(
    state: CupratedRpcHandler,
    request: FlushTransactionPoolRequest,
) -> FlushTransactionPoolResponse {
    todo!()
}

async fn get_output_histogram(
    state: CupratedRpcHandler,
    request: GetOutputHistogramRequest,
) -> GetOutputHistogramResponse {
    todo!()
}

async fn get_coinbase_tx_sum(
    state: CupratedRpcHandler,
    request: GetCoinbaseTxSumRequest,
) -> GetCoinbaseTxSumResponse {
    todo!()
}

async fn get_version(state: CupratedRpcHandler, request: GetVersionRequest) -> GetVersionResponse {
    todo!()
}

async fn get_fee_estimate(
    state: CupratedRpcHandler,
    request: GetFeeEstimateRequest,
) -> GetFeeEstimateResponse {
    todo!()
}

async fn get_alternate_chains(
    state: CupratedRpcHandler,
    request: GetAlternateChainsRequest,
) -> GetAlternateChainsResponse {
    todo!()
}

async fn relay_tx(state: CupratedRpcHandler, request: RelayTxRequest) -> RelayTxResponse {
    todo!()
}

async fn sync_info(state: CupratedRpcHandler, request: SyncInfoRequest) -> SyncInfoResponse {
    todo!()
}

async fn get_transaction_pool_backlog(
    state: CupratedRpcHandler,
    request: GetTransactionPoolBacklogRequest,
) -> GetTransactionPoolBacklogResponse {
    todo!()
}

async fn get_miner_data(
    state: CupratedRpcHandler,
    request: GetMinerDataRequest,
) -> GetMinerDataResponse {
    todo!()
}

async fn prune_blockchain(
    state: CupratedRpcHandler,
    request: PruneBlockchainRequest,
) -> PruneBlockchainResponse {
    todo!()
}

async fn calc_pow(state: CupratedRpcHandler, request: CalcPowRequest) -> CalcPowResponse {
    todo!()
}

async fn flush_cache(state: CupratedRpcHandler, request: FlushCacheRequest) -> FlushCacheResponse {
    todo!()
}

async fn add_aux_pow(state: CupratedRpcHandler, request: AddAuxPowRequest) -> AddAuxPowResponse {
    todo!()
}

async fn get_tx_ids_loose(
    state: CupratedRpcHandler,
    request: GetTxIdsLooseRequest,
) -> GetTxIdsLooseResponse {
    todo!()
}
