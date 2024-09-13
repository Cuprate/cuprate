use std::sync::Arc;

use anyhow::{anyhow, Error};
use futures::StreamExt;
use monero_serai::block::Block;
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
    misc::{BlockHeader, Status},
    CORE_RPC_VERSION,
};
use cuprate_types::{blockchain::BlockchainReadRequest, Chain, HardFork};

use crate::{
    rpc::{
        blockchain, helper, CupratedRpcHandlerState, RESTRICTED_BLOCK_COUNT,
        RESTRICTED_BLOCK_HEADER_RANGE,
    },
    version::CUPRATED_VERSION_IS_RELEASE,
};

use super::constants::BLOCK_SIZE_SANITY_LEEWAY;

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

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1790-L1804>
async fn get_block_count(
    mut state: CupratedRpcHandlerState,
    request: GetBlockCountRequest,
) -> Result<GetBlockCountResponse, Error> {
    Ok(GetBlockCountResponse {
        base: ResponseBase::ok(),
        count: helper::top_height(&mut state).await?.0,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1806-L1831>
async fn on_get_block_hash(
    mut state: CupratedRpcHandlerState,
    request: OnGetBlockHashRequest,
) -> Result<OnGetBlockHashResponse, Error> {
    let [height] = request.block_height;
    let hash = blockchain::block_hash(&mut state, height).await?;
    let block_hash = hex::encode(hash);

    Ok(OnGetBlockHashResponse { block_hash })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2209-L2266>
async fn submit_block(
    mut state: CupratedRpcHandlerState,
    request: SubmitBlockRequest,
) -> Result<SubmitBlockResponse, Error> {
    let [blob] = request.block_blob;

    let limit = blockchain::cumulative_block_weight_limit(&mut state).await?;

    if blob.len() > limit + BLOCK_SIZE_SANITY_LEEWAY {
        return Err(anyhow!("Block size is too big, rejecting block"));
    }

    let bytes = hex::decode(blob)?;
    let block = Block::read(&mut bytes.as_slice())?;

    // <https://github.com/monero-project/monero/blob/master/src/cryptonote_core/cryptonote_core.cpp#L1540>
    let block_id = todo!("submit block to DB");
    todo!("relay to P2P");
    todo!("send to txpool");

    Ok(SubmitBlockResponse {
        base: ResponseBase::ok(),
        block_id,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2268-L2340>
async fn generate_blocks(
    state: CupratedRpcHandlerState,
    request: GenerateBlocksRequest,
) -> Result<GenerateBlocksResponse, Error> {
    Ok(GenerateBlocksResponse {
        base: ResponseBase::ok(),
        blocks: todo!(),
        height: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2468-L2498>
async fn get_last_block_header(
    mut state: CupratedRpcHandlerState,
    request: GetLastBlockHeaderRequest,
) -> Result<GetLastBlockHeaderResponse, Error> {
    let (height, _) = helper::top_height(&mut state).await?;
    let block_header = helper::block_header(&mut state, height, request.fill_pow_hash).await?;

    Ok(GetLastBlockHeaderResponse {
        base: AccessResponseBase::ok(),
        block_header,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2468-L2498>
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
        let hash = helper::hex_to_hash(hex)?;
        let block_header = helper::block_header_by_hash(state, hash, fill_pow_hash).await?;
        Ok(block_header)
    }

    let block_header = get(&mut state, request.hash, request.fill_pow_hash).await?;

    // FIXME PERF: could make a `Vec` on await on all tasks at the same time.
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

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2629-L2662>
async fn get_block_header_by_height(
    mut state: CupratedRpcHandlerState,
    request: GetBlockHeaderByHeightRequest,
) -> Result<GetBlockHeaderByHeightResponse, Error> {
    helper::check_height(&mut state, request.height).await?;
    let block_header =
        helper::block_header(&mut state, request.height, request.fill_pow_hash).await?;

    Ok(GetBlockHeaderByHeightResponse {
        base: AccessResponseBase::ok(),
        block_header,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2569-L2627>
async fn get_block_headers_range(
    mut state: CupratedRpcHandlerState,
    request: GetBlockHeadersRangeRequest,
) -> Result<GetBlockHeadersRangeResponse, Error> {
    let (top_height, _) = helper::top_height(&mut state).await?;

    if request.start_height >= top_height
        || request.end_height >= top_height
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
        let ready = state.blockchain_read.ready().await?;
        for height in request.start_height..=request.end_height {
            let height = u64_to_usize(height);
            let task = tokio::task::spawn(ready.call(BlockchainReadRequest::Block(height)));
            tasks.push(task);
        }
    }

    for task in tasks {
        let BlockchainResponse::Block(header) = task.await?? else {
            unreachable!();
        };
        // headers.push((&header).into());
        headers.push(todo!());
    }

    Ok(GetBlockHeadersRangeResponse {
        base: AccessResponseBase::ok(),
        headers,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2664-L2727>
async fn get_block(
    mut state: CupratedRpcHandlerState,
    request: GetBlockRequest,
) -> Result<GetBlockResponse, Error> {
    let block = if request.hash.is_empty() {
        helper::check_height(&mut state, request.height).await?;
        blockchain::block(&mut state, request.height).await?
    } else {
        let hash = helper::hex_to_hash(request.hash)?;
        blockchain::block_by_hash(&mut state, hash).await?
    };

    Ok(todo!())

    // let block_header = (&block).into();
    // let blob = hex::encode(block.block_blob);
    // let miner_tx_hash = hex::encode(block.block.miner_transaction.hash());
    // let tx_hashes = block
    //     .txs
    //     .into_iter()
    //     .map(|tx| hex::encode(tx.tx_hash))
    //     .collect();

    // Ok(GetBlockResponse {
    //     base: AccessResponseBase::ok(),
    //     blob,
    //     json: todo!(), // TODO: make `JSON` type in `cuprate_rpc_types`
    //     miner_tx_hash,
    //     tx_hashes,
    //     block_header,
    // })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2729-L2738>
async fn get_connections(
    state: CupratedRpcHandlerState,
    request: GetConnectionsRequest,
) -> Result<GetConnectionsResponse, Error> {
    Ok(GetConnectionsResponse {
        base: ResponseBase::ok(),
        connections: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L501-L582>
async fn get_info(
    state: CupratedRpcHandlerState,
    request: GetInfoRequest,
) -> Result<GetInfoResponse, Error> {
    Ok(GetInfoResponse {
        base: AccessResponseBase::ok(),
        adjusted_time: todo!(),
        alt_blocks_count: todo!(),
        block_size_limit: todo!(),
        block_size_median: todo!(),
        block_weight_limit: todo!(),
        block_weight_median: todo!(),
        bootstrap_daemon_address: todo!(),
        busy_syncing: todo!(),
        cumulative_difficulty_top64: todo!(),
        cumulative_difficulty: todo!(),
        database_size: todo!(),
        difficulty_top64: todo!(),
        difficulty: todo!(),
        free_space: todo!(),
        grey_peerlist_size: todo!(),
        height: todo!(),
        height_without_bootstrap: todo!(),
        incoming_connections_count: todo!(),
        mainnet: todo!(),
        nettype: todo!(),
        offline: todo!(),
        outgoing_connections_count: todo!(),
        restricted: todo!(),
        rpc_connections_count: todo!(),
        stagenet: todo!(),
        start_time: todo!(),
        synchronized: todo!(),
        target_height: todo!(),
        target: todo!(),
        testnet: todo!(),
        top_block_hash: todo!(),
        tx_count: todo!(),
        tx_pool_size: todo!(),
        update_available: todo!(),
        version: todo!(),
        was_bootstrap_ever_used: todo!(),
        white_peerlist_size: todo!(),
        wide_cumulative_difficulty: todo!(),
        wide_difficulty: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2751-L2766>
async fn hard_fork_info(
    mut state: CupratedRpcHandlerState,
    request: HardForkInfoRequest,
) -> Result<HardForkInfoResponse, Error> {
    let hard_fork = if request.version > 0 {
        HardFork::from_version(request.version)?
    } else {
        blockchain::current_hard_fork(&mut state).await?
    };

    Ok(HardForkInfoResponse {
        base: AccessResponseBase::ok(),
        earliest_height: todo!(),
        enabled: hard_fork.is_current(),
        state: todo!(),
        threshold: todo!(),
        version: hard_fork.as_u8(),
        votes: todo!(),
        voting: todo!(),
        window: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2832-L2878>
async fn set_bans(
    state: CupratedRpcHandlerState,
    request: SetBansRequest,
) -> Result<SetBansResponse, Error> {
    todo!();

    Ok(SetBansResponse {
        base: ResponseBase::ok(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2768-L2801>
async fn get_bans(
    state: CupratedRpcHandlerState,
    request: GetBansRequest,
) -> Result<GetBansResponse, Error> {
    Ok(GetBansResponse {
        base: ResponseBase::ok(),
        bans: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2803-L2830>
async fn banned(
    state: CupratedRpcHandlerState,
    request: BannedRequest,
) -> Result<BannedResponse, Error> {
    Ok(BannedResponse {
        banned: todo!(),
        seconds: todo!(),
        status: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2880-L2932>
async fn flush_transaction_pool(
    state: CupratedRpcHandlerState,
    request: FlushTransactionPoolRequest,
) -> Result<FlushTransactionPoolResponse, Error> {
    todo!();
    Ok(FlushTransactionPoolResponse { status: Status::Ok })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2934-L2979>
async fn get_output_histogram(
    state: CupratedRpcHandlerState,
    request: GetOutputHistogramRequest,
) -> Result<GetOutputHistogramResponse, Error> {
    Ok(GetOutputHistogramResponse {
        base: AccessResponseBase::ok(),
        histogram: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2998-L3013>
async fn get_coinbase_tx_sum(
    state: CupratedRpcHandlerState,
    request: GetCoinbaseTxSumRequest,
) -> Result<GetCoinbaseTxSumResponse, Error> {
    Ok(GetCoinbaseTxSumResponse {
        base: AccessResponseBase::ok(),
        emission_amount: todo!(),
        emission_amount_top64: todo!(),
        fee_amount: todo!(),
        fee_amount_top64: todo!(),
        wide_emission_amount: todo!(),
        wide_fee_amount: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2981-L2996>
async fn get_version(
    mut state: CupratedRpcHandlerState,
    request: GetVersionRequest,
) -> Result<GetVersionResponse, Error> {
    Ok(GetVersionResponse {
        base: ResponseBase::ok(),
        version: CORE_RPC_VERSION,
        release: CUPRATED_VERSION_IS_RELEASE,
        current_height: helper::top_height(&mut state).await?.0,
        target_height: todo!(),
        hard_forks: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3015-L3031>
async fn get_fee_estimate(
    state: CupratedRpcHandlerState,
    request: GetFeeEstimateRequest,
) -> Result<GetFeeEstimateResponse, Error> {
    Ok(GetFeeEstimateResponse {
        base: AccessResponseBase::ok(),
        fee: todo!(),
        fees: todo!(),
        quantization_mask: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3033-L3064>
async fn get_alternate_chains(
    state: CupratedRpcHandlerState,
    request: GetAlternateChainsRequest,
) -> Result<GetAlternateChainsResponse, Error> {
    Ok(GetAlternateChainsResponse {
        base: ResponseBase::ok(),
        chains: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3254-L3304>
async fn relay_tx(
    state: CupratedRpcHandlerState,
    request: RelayTxRequest,
) -> Result<RelayTxResponse, Error> {
    todo!();
    Ok(RelayTxResponse { status: Status::Ok })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3306-L3330>
async fn sync_info(
    state: CupratedRpcHandlerState,
    request: SyncInfoRequest,
) -> Result<SyncInfoResponse, Error> {
    Ok(SyncInfoResponse {
        base: AccessResponseBase::ok(),
        height: todo!(),
        next_needed_pruning_seed: todo!(),
        overview: todo!(),
        peers: todo!(),
        spans: todo!(),
        target_height: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3332-L3350>
async fn get_transaction_pool_backlog(
    state: CupratedRpcHandlerState,
    request: GetTransactionPoolBacklogRequest,
) -> Result<GetTransactionPoolBacklogResponse, Error> {
    Ok(GetTransactionPoolBacklogResponse {
        base: ResponseBase::ok(),
        backlog: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1998-L2033>
async fn get_miner_data(
    state: CupratedRpcHandlerState,
    request: GetMinerDataRequest,
) -> Result<GetMinerDataResponse, Error> {
    Ok(GetMinerDataResponse {
        base: ResponseBase::ok(),
        major_version: todo!(),
        height: todo!(),
        prev_id: todo!(),
        seed_hash: todo!(),
        difficulty: todo!(),
        median_weight: todo!(),
        already_generated_coins: todo!(),
        tx_backlog: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3453-L3476>
async fn prune_blockchain(
    state: CupratedRpcHandlerState,
    request: PruneBlockchainRequest,
) -> Result<PruneBlockchainResponse, Error> {
    Ok(PruneBlockchainResponse {
        base: ResponseBase::ok(),
        pruned: todo!(),
        pruning_seed: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2035-L2070>
async fn calc_pow(
    state: CupratedRpcHandlerState,
    request: CalcPowRequest,
) -> Result<CalcPowResponse, Error> {
    Ok(CalcPowResponse { pow_hash: todo!() })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3542-L3551>
async fn flush_cache(
    state: CupratedRpcHandlerState,
    request: FlushCacheRequest,
) -> Result<FlushCacheResponse, Error> {
    todo!();

    Ok(FlushCacheResponse {
        base: ResponseBase::ok(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2072-L2207>
async fn add_aux_pow(
    state: CupratedRpcHandlerState,
    request: AddAuxPowRequest,
) -> Result<AddAuxPowResponse, Error> {
    Ok(AddAuxPowResponse {
        base: ResponseBase::ok(),
        blocktemplate_blob: todo!(),
        blockhashing_blob: todo!(),
        merkle_root: todo!(),
        merkle_tree_depth: todo!(),
        aux_pow: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3553-L3627>
async fn get_tx_ids_loose(
    state: CupratedRpcHandlerState,
    request: GetTxIdsLooseRequest,
) -> Result<GetTxIdsLooseResponse, Error> {
    Ok(GetTxIdsLooseResponse {
        base: ResponseBase::ok(),
        txids: todo!(),
    })
}