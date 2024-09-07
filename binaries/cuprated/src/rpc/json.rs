use std::sync::Arc;

use anyhow::Error;
use tower::ServiceExt;

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

use crate::rpc::CupratedRpcHandlerState;

/// Map a [`JsonRpcRequest`] to the function that will lead to a [`JsonRpcResponse`].
pub(super) async fn map_request(
    state: CupratedRpcHandlerState,
    request: JsonRpcRequest,
) -> Result<JsonRpcResponse, Error> {
    use JsonRpcRequest as Req;
    use JsonRpcResponse as Resp;

    Ok(match request {
        Req::GetBlockCount(r) => Resp::GetBlockCount(get_block_count(state, r).await?),
        Req::OnGetBlockHash(r) => Resp::OnGetBlockHash(on_get_block_hash(state, r).await?),
        Req::SubmitBlock(r) => Resp::SubmitBlock(submit_block(state, r).await?),
        Req::GenerateBlocks(r) => Resp::GenerateBlocks(generate_blocks(state, r).await?),
        Req::GetLastBlockHeader(r) => {
            Resp::GetLastBlockHeader(get_last_block_header(state, r).await?)
        }
        Req::GetBlockHeaderByHash(r) => {
            Resp::GetBlockHeaderByHash(get_block_header_by_hash(state, r).await?)
        }
        Req::GetBlockHeaderByHeight(r) => {
            Resp::GetBlockHeaderByHeight(get_block_header_by_height(state, r).await?)
        }
        Req::GetBlockHeadersRange(r) => {
            Resp::GetBlockHeadersRange(get_block_headers_range(state, r).await?)
        }
        Req::GetBlock(r) => Resp::GetBlock(get_block(state, r).await?),
        Req::GetConnections(r) => Resp::GetConnections(get_connections(state, r).await?),
        Req::GetInfo(r) => Resp::GetInfo(get_info(state, r).await?),
        Req::HardForkInfo(r) => Resp::HardForkInfo(hard_fork_info(state, r).await?),
        Req::SetBans(r) => Resp::SetBans(set_bans(state, r).await?),
        Req::GetBans(r) => Resp::GetBans(get_bans(state, r).await?),
        Req::Banned(r) => Resp::Banned(banned(state, r).await?),
        Req::FlushTransactionPool(r) => {
            Resp::FlushTransactionPool(flush_transaction_pool(state, r).await?)
        }
        Req::GetOutputHistogram(r) => {
            Resp::GetOutputHistogram(get_output_histogram(state, r).await?)
        }
        Req::GetCoinbaseTxSum(r) => Resp::GetCoinbaseTxSum(get_coinbase_tx_sum(state, r).await?),
        Req::GetVersion(r) => Resp::GetVersion(get_version(state, r).await?),
        Req::GetFeeEstimate(r) => Resp::GetFeeEstimate(get_fee_estimate(state, r).await?),
        Req::GetAlternateChains(r) => {
            Resp::GetAlternateChains(get_alternate_chains(state, r).await?)
        }
        Req::RelayTx(r) => Resp::RelayTx(relay_tx(state, r).await?),
        Req::SyncInfo(r) => Resp::SyncInfo(sync_info(state, r).await?),
        Req::GetTransactionPoolBacklog(r) => {
            Resp::GetTransactionPoolBacklog(get_transaction_pool_backlog(state, r).await?)
        }
        Req::GetMinerData(r) => Resp::GetMinerData(get_miner_data(state, r).await?),
        Req::PruneBlockchain(r) => Resp::PruneBlockchain(prune_blockchain(state, r).await?),
        Req::CalcPow(r) => Resp::CalcPow(calc_pow(state, r).await?),
        Req::FlushCache(r) => Resp::FlushCache(flush_cache(state, r).await?),
        Req::AddAuxPow(r) => Resp::AddAuxPow(add_aux_pow(state, r).await?),
        Req::GetTxIdsLoose(r) => Resp::GetTxIdsLoose(get_tx_ids_loose(state, r).await?),
    })
}

async fn get_block_count(
    state: CupratedRpcHandlerState,
    request: GetBlockCountRequest,
) -> Result<GetBlockCountResponse, Error> {
    todo!()
}

async fn on_get_block_hash(
    state: CupratedRpcHandlerState,
    request: OnGetBlockHashRequest,
) -> Result<OnGetBlockHashResponse, Error> {
    todo!()
}

async fn submit_block(
    state: CupratedRpcHandlerState,
    request: SubmitBlockRequest,
) -> Result<SubmitBlockResponse, Error> {
    todo!()
}

async fn generate_blocks(
    state: CupratedRpcHandlerState,
    request: GenerateBlocksRequest,
) -> Result<GenerateBlocksResponse, Error> {
    todo!()
}

async fn get_last_block_header(
    state: CupratedRpcHandlerState,
    request: GetLastBlockHeaderRequest,
) -> Result<GetLastBlockHeaderResponse, Error> {
    todo!()
}

async fn get_block_header_by_hash(
    state: CupratedRpcHandlerState,
    request: GetBlockHeaderByHashRequest,
) -> Result<GetBlockHeaderByHashResponse, Error> {
    todo!()
}

async fn get_block_header_by_height(
    state: CupratedRpcHandlerState,
    request: GetBlockHeaderByHeightRequest,
) -> Result<GetBlockHeaderByHeightResponse, Error> {
    todo!()
}

async fn get_block_headers_range(
    state: CupratedRpcHandlerState,
    request: GetBlockHeadersRangeRequest,
) -> Result<GetBlockHeadersRangeResponse, Error> {
    todo!()
}

async fn get_block(
    state: CupratedRpcHandlerState,
    request: GetBlockRequest,
) -> Result<GetBlockResponse, Error> {
    todo!()
}

async fn get_connections(
    state: CupratedRpcHandlerState,
    request: GetConnectionsRequest,
) -> Result<GetConnectionsResponse, Error> {
    todo!()
}

async fn get_info(
    state: CupratedRpcHandlerState,
    request: GetInfoRequest,
) -> Result<GetInfoResponse, Error> {
    todo!()
}

async fn hard_fork_info(
    state: CupratedRpcHandlerState,
    request: HardForkInfoRequest,
) -> Result<HardForkInfoResponse, Error> {
    todo!()
}

async fn set_bans(
    state: CupratedRpcHandlerState,
    request: SetBansRequest,
) -> Result<SetBansResponse, Error> {
    todo!()
}

async fn get_bans(
    state: CupratedRpcHandlerState,
    request: GetBansRequest,
) -> Result<GetBansResponse, Error> {
    todo!()
}

async fn banned(
    state: CupratedRpcHandlerState,
    request: BannedRequest,
) -> Result<BannedResponse, Error> {
    todo!()
}

async fn flush_transaction_pool(
    state: CupratedRpcHandlerState,
    request: FlushTransactionPoolRequest,
) -> Result<FlushTransactionPoolResponse, Error> {
    todo!()
}

async fn get_output_histogram(
    state: CupratedRpcHandlerState,
    request: GetOutputHistogramRequest,
) -> Result<GetOutputHistogramResponse, Error> {
    todo!()
}

async fn get_coinbase_tx_sum(
    state: CupratedRpcHandlerState,
    request: GetCoinbaseTxSumRequest,
) -> Result<GetCoinbaseTxSumResponse, Error> {
    todo!()
}

async fn get_version(
    state: CupratedRpcHandlerState,
    request: GetVersionRequest,
) -> Result<GetVersionResponse, Error> {
    todo!()
}

async fn get_fee_estimate(
    state: CupratedRpcHandlerState,
    request: GetFeeEstimateRequest,
) -> Result<GetFeeEstimateResponse, Error> {
    todo!()
}

async fn get_alternate_chains(
    state: CupratedRpcHandlerState,
    request: GetAlternateChainsRequest,
) -> Result<GetAlternateChainsResponse, Error> {
    todo!()
}

async fn relay_tx(
    state: CupratedRpcHandlerState,
    request: RelayTxRequest,
) -> Result<RelayTxResponse, Error> {
    todo!()
}

async fn sync_info(
    state: CupratedRpcHandlerState,
    request: SyncInfoRequest,
) -> Result<SyncInfoResponse, Error> {
    todo!()
}

async fn get_transaction_pool_backlog(
    state: CupratedRpcHandlerState,
    request: GetTransactionPoolBacklogRequest,
) -> Result<GetTransactionPoolBacklogResponse, Error> {
    todo!()
}

async fn get_miner_data(
    state: CupratedRpcHandlerState,
    request: GetMinerDataRequest,
) -> Result<GetMinerDataResponse, Error> {
    todo!()
}

async fn prune_blockchain(
    state: CupratedRpcHandlerState,
    request: PruneBlockchainRequest,
) -> Result<PruneBlockchainResponse, Error> {
    todo!()
}

async fn calc_pow(
    state: CupratedRpcHandlerState,
    request: CalcPowRequest,
) -> Result<CalcPowResponse, Error> {
    todo!()
}

async fn flush_cache(
    state: CupratedRpcHandlerState,
    request: FlushCacheRequest,
) -> Result<FlushCacheResponse, Error> {
    todo!()
}

async fn add_aux_pow(
    state: CupratedRpcHandlerState,
    request: AddAuxPowRequest,
) -> Result<AddAuxPowResponse, Error> {
    todo!()
}

async fn get_tx_ids_loose(
    state: CupratedRpcHandlerState,
    request: GetTxIdsLooseRequest,
) -> Result<GetTxIdsLooseResponse, Error> {
    todo!()
}
