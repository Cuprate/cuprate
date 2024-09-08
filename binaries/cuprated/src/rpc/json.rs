use std::sync::Arc;

use anyhow::{anyhow, Error};
use futures::StreamExt;
use tower::{Service, ServiceExt};

use cuprate_consensus::BlockchainResponse;
use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_rpc_types::{
    base::{AccessResponseBase, ResponseBase},
    json::{
        AddAuxPowRequest, AddAuxPowResponse, BannedRequest, BannedResponse, CalcPowRequest,
        CalcPowResponse, FlushCacheRequest, FlushCacheResponse, FlushTransactionPoolRequest,
        FlushTransactionPoolResponse, GenerateBlocksRequest, GenerateBlocksResponse,
        GetAlternateChainsRequest, GetAlternateChainsResponse, GetBansRequest, GetBansResponse,
        GetBlockCountRequest, GetBlockCountResponse, GetBlockHeaderByHashRequest,
        GetBlockHeaderByHashResponse, GetBlockHeaderByHeightRequest,
        GetBlockHeaderByHeightResponse, GetBlockHeadersRangeRequest, GetBlockHeadersRangeResponse,
        GetBlockRequest, GetBlockResponse, GetCoinbaseTxSumRequest, GetCoinbaseTxSumResponse,
        GetConnectionsRequest, GetConnectionsResponse, GetFeeEstimateRequest,
        GetFeeEstimateResponse, GetInfoRequest, GetInfoResponse, GetLastBlockHeaderRequest,
        GetLastBlockHeaderResponse, GetMinerDataRequest, GetMinerDataResponse,
        GetOutputHistogramRequest, GetOutputHistogramResponse, GetTransactionPoolBacklogRequest,
        GetTransactionPoolBacklogResponse, GetTxIdsLooseRequest, GetTxIdsLooseResponse,
        GetVersionRequest, GetVersionResponse, HardForkInfoRequest, HardForkInfoResponse,
        JsonRpcRequest, JsonRpcResponse, OnGetBlockHashRequest, OnGetBlockHashResponse,
        PruneBlockchainRequest, PruneBlockchainResponse, RelayTxRequest, RelayTxResponse,
        SetBansRequest, SetBansResponse, SubmitBlockRequest, SubmitBlockResponse, SyncInfoRequest,
        SyncInfoResponse,
    },
    misc::BlockHeader,
};
use cuprate_types::{blockchain::BlockchainReadRequest, Chain};

use crate::rpc::{CupratedRpcHandlerState, RESTRICTED_BLOCK_COUNT, RESTRICTED_BLOCK_HEADER_RANGE};

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
    let BlockchainResponse::ChainHeight(count, hash) = state
        .blockchain
        .oneshot(BlockchainReadRequest::ChainHeight)
        .await?
    else {
        unreachable!();
    };

    Ok(GetBlockCountResponse {
        base: ResponseBase::ok(),
        count: usize_to_u64(count),
    })
}

async fn on_get_block_hash(
    state: CupratedRpcHandlerState,
    request: OnGetBlockHashRequest,
) -> Result<OnGetBlockHashResponse, Error> {
    let BlockchainResponse::BlockHash(hash) = state
        .blockchain
        .oneshot(BlockchainReadRequest::BlockHash(
            u64_to_usize(request.block_height[0]),
            Chain::Main,
        ))
        .await?
    else {
        unreachable!();
    };

    Ok(OnGetBlockHashResponse {
        block_hash: hex::encode(hash),
    })
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
    mut state: CupratedRpcHandlerState,
    request: GetBlockHeaderByHashRequest,
) -> Result<GetBlockHeaderByHashResponse, Error> {
    if state.restricted && request.hashes.len() > RESTRICTED_BLOCK_COUNT {
        return Err(anyhow!(
            "Too many block headers requested in restricted mode"
        ));
    }

    async fn get(
        state: &mut CupratedRpcHandlerState,
        hex: String,
        fill_pow_hash: bool,
    ) -> Result<BlockHeader, Error> {
        let Ok(bytes) = hex::decode(&hex) else {
            return Err(anyhow!(
                "Failed to parse hex representation of block hash. Hex = {hex}."
            ));
        };

        let Ok(hash) = bytes.try_into() else {
            return Err(anyhow!("TODO"));
        };

        let BlockchainResponse::BlockByHash(block) = state
            .blockchain
            .ready()
            .await?
            .call(BlockchainReadRequest::BlockByHash(hash))
            .await?
        else {
            unreachable!();
        };

        let block_header = BlockHeader::from(&block);

        Ok(block_header)
    }

    let block_header = get(&mut state, request.hash, request.fill_pow_hash).await?;

    let mut block_headers = Vec::with_capacity(request.hashes.len());
    for hash in request.hashes {
        let hash = get(&mut state, hash, request.fill_pow_hash).await?;
        block_headers.push(hash);
    }

    Ok(GetBlockHeaderByHashResponse {
        base: AccessResponseBase::ok(),
        block_header,
        block_headers,
    })
}

async fn get_block_header_by_height(
    mut state: CupratedRpcHandlerState,
    request: GetBlockHeaderByHeightRequest,
) -> Result<GetBlockHeaderByHeightResponse, Error> {
    let BlockchainResponse::ChainHeight(chain_height, _) = state
        .blockchain
        .ready()
        .await?
        .call(BlockchainReadRequest::ChainHeight)
        .await?
    else {
        unreachable!();
    };

    let height = chain_height.saturating_sub(1);

    if request.height > usize_to_u64(height) {
        return Err(anyhow!(
            "Requested block height: {} greater than current top block height: {height}",
            request.height
        ));
    }

    let BlockchainResponse::Block(block) = state
        .blockchain
        .ready()
        .await?
        .call(BlockchainReadRequest::Block(height))
        .await?
    else {
        unreachable!();
    };

    Ok(GetBlockHeaderByHeightResponse {
        base: AccessResponseBase::ok(),
        block_header: BlockHeader::from(&block),
    })
}

async fn get_block_headers_range(
    mut state: CupratedRpcHandlerState,
    request: GetBlockHeadersRangeRequest,
) -> Result<GetBlockHeadersRangeResponse, Error> {
    let BlockchainResponse::ChainHeight(chain_height, _) = state
        .blockchain
        .ready()
        .await?
        .call(BlockchainReadRequest::ChainHeight)
        .await?
    else {
        unreachable!();
    };

    let height = chain_height.saturating_sub(1);
    let height_u64 = usize_to_u64(height);

    if request.start_height >= height_u64
        || request.end_height >= height_u64
        || request.start_height > request.end_height
    {
        return Err(anyhow!("Invalid start/end heights"));
    }

    if state.restricted
        && request.end_height.saturating_sub(request.start_height) + 1
            > RESTRICTED_BLOCK_HEADER_RANGE
    {
        return Err(anyhow!("Too many block headers requested."));
    }

    let block_len = u64_to_usize(request.end_height.saturating_sub(request.start_height));
    let mut tasks = Vec::with_capacity(block_len);
    let mut headers = Vec::with_capacity(block_len);

    {
        let ready = state.blockchain.ready().await?;
        for block in request.start_height..=request.end_height {
            let task = tokio::task::spawn(ready.call(BlockchainReadRequest::Block(height)));
            tasks.push(task);
        }
    }

    for task in tasks {
        let BlockchainResponse::Block(header) = task.await?? else {
            unreachable!();
        };
        headers.push((&header).into());
    }

    Ok(GetBlockHeadersRangeResponse {
        base: AccessResponseBase::ok(),
        headers,
    })
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
