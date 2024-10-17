use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Error};
use cuprate_p2p_core::{client::handshaker::builder::DummyAddressBook, ClearNet};
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use cuprate_consensus::{BlockchainReadRequest, BlockchainResponse};
use cuprate_constants::{
    build::RELEASE,
    rpc::{RESTRICTED_BLOCK_COUNT, RESTRICTED_BLOCK_HEADER_RANGE},
};
use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_rpc_interface::RpcHandler;
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
    misc::{
        AuxPow, BlockHeader, ChainInfo, GetBan, GetMinerDataTxBacklogEntry, HardforkEntry,
        HistogramEntry, Status, SyncInfoPeer, TxBacklogEntry,
    },
    CORE_RPC_VERSION,
};
use cuprate_types::{Chain, HardFork};

use crate::{
    constants::VERSION_BUILD,
    rpc::{
        helper,
        request::{address_book, blockchain, blockchain_context, blockchain_manager, txpool},
        CupratedRpcHandler,
    },
    statics::START_INSTANT_UNIX,
};

/// Map a [`JsonRpcRequest`] to the function that will lead to a [`JsonRpcResponse`].
pub(super) async fn map_request(
    state: CupratedRpcHandler,
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
    mut state: CupratedRpcHandler,
    request: GetBlockCountRequest,
) -> Result<GetBlockCountResponse, Error> {
    Ok(GetBlockCountResponse {
        base: ResponseBase::OK,
        count: helper::top_height(&mut state).await?.0,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1806-L1831>
async fn on_get_block_hash(
    mut state: CupratedRpcHandler,
    request: OnGetBlockHashRequest,
) -> Result<OnGetBlockHashResponse, Error> {
    let [height] = request.block_height;
    let hash = blockchain::block_hash(&mut state.blockchain_read, height, todo!("access to chain"))
        .await?;
    let block_hash = hex::encode(hash);

    Ok(OnGetBlockHashResponse { block_hash })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2209-L2266>
async fn submit_block(
    mut state: CupratedRpcHandler,
    request: SubmitBlockRequest,
) -> Result<SubmitBlockResponse, Error> {
    // Parse hex into block.
    let [blob] = request.block_blob;
    let bytes = hex::decode(blob)?;
    let block = Block::read(&mut bytes.as_slice())?;
    let block_id = hex::encode(block.hash());

    // Attempt to relay the block.
    blockchain_manager::relay_block(&mut state.blockchain_manager, block).await?;

    Ok(SubmitBlockResponse {
        base: ResponseBase::OK,
        block_id,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2268-L2340>
async fn generate_blocks(
    state: CupratedRpcHandler,
    request: GenerateBlocksRequest,
) -> Result<GenerateBlocksResponse, Error> {
    if todo!("active cuprated chain") != todo!("regtest chain") {
        return Err(anyhow!("Regtest required when generating blocks"));
    }

    // TODO: is this value only used as a local variable in the handler?
    // it may not be needed in the request type.
    let prev_block = helper::hex_to_hash(request.prev_block)?;

    let (blocks, height) = blockchain_manager::generate_blocks(
        &mut state.blockchain_manager,
        request.amount_of_blocks,
        prev_block,
        request.starting_nonce,
        request.wallet_address,
    )
    .await?;

    let blocks = blocks.into_iter().map(hex::encode).collect();

    Ok(GenerateBlocksResponse {
        base: ResponseBase::OK,
        blocks,
        height,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2468-L2498>
async fn get_last_block_header(
    mut state: CupratedRpcHandler,
    request: GetLastBlockHeaderRequest,
) -> Result<GetLastBlockHeaderResponse, Error> {
    let (height, _) = helper::top_height(&mut state).await?;
    let block_header = helper::block_header(&mut state, height, request.fill_pow_hash).await?;

    Ok(GetLastBlockHeaderResponse {
        base: AccessResponseBase::OK,
        block_header,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2500-L2567>
async fn get_block_header_by_hash(
    mut state: CupratedRpcHandler,
    request: GetBlockHeaderByHashRequest,
) -> Result<GetBlockHeaderByHashResponse, Error> {
    if state.restricted() && request.hashes.len() > RESTRICTED_BLOCK_COUNT {
        return Err(anyhow!(
            "Too many block headers requested in restricted mode"
        ));
    }

    async fn get(
        state: &mut CupratedRpcHandler,
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
        base: AccessResponseBase::OK,
        block_header,
        block_headers,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2629-L2662>
async fn get_block_header_by_height(
    mut state: CupratedRpcHandler,
    request: GetBlockHeaderByHeightRequest,
) -> Result<GetBlockHeaderByHeightResponse, Error> {
    helper::check_height(&mut state, request.height).await?;
    let block_header =
        helper::block_header(&mut state, request.height, request.fill_pow_hash).await?;

    Ok(GetBlockHeaderByHeightResponse {
        base: AccessResponseBase::OK,
        block_header,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2569-L2627>
async fn get_block_headers_range(
    mut state: CupratedRpcHandler,
    request: GetBlockHeadersRangeRequest,
) -> Result<GetBlockHeadersRangeResponse, Error> {
    let (top_height, _) = helper::top_height(&mut state).await?;

    if request.start_height >= top_height
        || request.end_height >= top_height
        || request.start_height > request.end_height
    {
        return Err(anyhow!("Invalid start/end heights"));
    }

    if state.restricted()
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
            let task = tokio::task::spawn(ready.call(BlockchainReadRequest::Block { height }));
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
        base: AccessResponseBase::OK,
        headers,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2664-L2727>
async fn get_block(
    mut state: CupratedRpcHandler,
    request: GetBlockRequest,
) -> Result<GetBlockResponse, Error> {
    let (block, block_header) = if request.hash.is_empty() {
        helper::check_height(&mut state, request.height).await?;
        let block = blockchain::block(&mut state.blockchain_read, request.height).await?;
        let block_header =
            helper::block_header(&mut state, request.height, request.fill_pow_hash).await?;
        (block, block_header)
    } else {
        let hash = helper::hex_to_hash(request.hash)?;
        let block = blockchain::block_by_hash(&mut state.blockchain_read, hash).await?;
        let block_header =
            helper::block_header_by_hash(&mut state, hash, request.fill_pow_hash).await?;
        (block, block_header)
    };

    let blob = hex::encode(block.serialize());
    let miner_tx_hash = hex::encode(block.miner_transaction.hash());
    let tx_hashes = block.transactions.iter().map(hex::encode).collect();
    let json = {
        let block = cuprate_types::json::block::Block::from(block);
        serde_json::to_string_pretty(&block)?
    };

    Ok(GetBlockResponse {
        base: AccessResponseBase::OK,
        blob,
        json,
        miner_tx_hash,
        tx_hashes,
        block_header,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2729-L2738>
async fn get_connections(
    state: CupratedRpcHandler,
    request: GetConnectionsRequest,
) -> Result<GetConnectionsResponse, Error> {
    let connections = address_book::connection_info::<ClearNet>(&mut DummyAddressBook).await?;

    Ok(GetConnectionsResponse {
        base: ResponseBase::OK,
        connections,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L501-L582>
async fn get_info(
    mut state: CupratedRpcHandler,
    request: GetInfoRequest,
) -> Result<GetInfoResponse, Error> {
    let restricted = state.restricted();
    let context = blockchain_context::context(&mut state.blockchain_context).await?;

    let c = context.unchecked_blockchain_context();
    let cumulative_difficulty = c.cumulative_difficulty;
    let adjusted_time = c.current_adjusted_timestamp_for_time_lock(); // TODO: is this correct?

    let c = &c.context_to_verify_block;

    let alt_blocks_count = if restricted {
        0
    } else {
        blockchain::alt_chain_count(&mut state.blockchain_read).await?
    };
    let block_weight_limit = usize_to_u64(c.effective_median_weight); // TODO: is this correct?
    let block_weight_median = usize_to_u64(c.median_weight_for_block_reward); // TODO: is this correct?
    let block_size_limit = block_weight_limit;
    let block_size_median = block_weight_median;
    let (bootstrap_daemon_address, was_bootstrap_ever_used) = if restricted {
        (String::new(), false)
    } else {
        todo!()
    };
    let busy_syncing = blockchain_manager::syncing(&mut state.blockchain_manager).await?;
    let (cumulative_difficulty, cumulative_difficulty_top64) =
        split_u128_into_low_high_bits(cumulative_difficulty);
    let (database_size, free_space) = blockchain::database_size(&mut state.blockchain_read).await?;
    let (database_size, free_space) = if restricted {
        let database_size = todo!(); // round_up(res.database_size, 5ull* 1024 * 1024 * 1024) */
        (database_size, u64::MAX)
    } else {
        (database_size, free_space)
    };
    let (difficulty, difficulty_top64) = split_u128_into_low_high_bits(c.next_difficulty);
    let height = usize_to_u64(c.chain_height);
    let height_without_bootstrap = if restricted { 0 } else { height };
    let (incoming_connections_count, outgoing_connections_count) = if restricted {
        (0, 0)
    } else {
        address_book::connection_count::<ClearNet>(&mut DummyAddressBook).await?
    };
    let mainnet = todo!();
    let nettype = todo!();
    let offline = todo!();
    let rpc_connections_count = if restricted { 0 } else { todo!() };
    let stagenet = todo!();
    let start_time = if restricted { 0 } else { *START_INSTANT_UNIX };
    let synchronized = blockchain_manager::synced(&mut state.blockchain_manager).await?;
    let target_height = blockchain_manager::target_height(&mut state.blockchain_manager).await?;
    let target = blockchain_manager::target(&mut state.blockchain_manager)
        .await?
        .as_secs();
    let testnet = todo!();
    let top_block_hash = hex::encode(c.top_hash);
    let tx_count = blockchain::total_tx_count(&mut state.blockchain_read).await?;
    let tx_pool_size = txpool::size(&mut state.txpool_read, !restricted).await?;
    let update_available = if restricted { false } else { todo!() };
    let version = if restricted {
        String::new()
    } else {
        VERSION_BUILD.to_string()
    };
    let (white_peerlist_size, grey_peerlist_size) = if restricted {
        (0, 0)
    } else {
        address_book::peerlist_size::<ClearNet>(&mut DummyAddressBook).await?
    };
    let wide_cumulative_difficulty = format!("{cumulative_difficulty:#x}");
    let wide_difficulty = format!("{:#x}", c.next_difficulty);

    Ok(GetInfoResponse {
        base: AccessResponseBase::OK,
        adjusted_time,
        alt_blocks_count,
        block_size_limit,
        block_size_median,
        block_weight_limit,
        block_weight_median,
        bootstrap_daemon_address,
        busy_syncing,
        cumulative_difficulty_top64,
        cumulative_difficulty,
        database_size,
        difficulty_top64,
        difficulty,
        free_space,
        grey_peerlist_size,
        height,
        height_without_bootstrap,
        incoming_connections_count,
        mainnet,
        nettype,
        offline,
        outgoing_connections_count,
        restricted,
        rpc_connections_count,
        stagenet,
        start_time,
        synchronized,
        target_height,
        target,
        testnet,
        top_block_hash,
        tx_count,
        tx_pool_size,
        update_available,
        version,
        was_bootstrap_ever_used,
        white_peerlist_size,
        wide_cumulative_difficulty,
        wide_difficulty,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2751-L2766>
async fn hard_fork_info(
    mut state: CupratedRpcHandler,
    request: HardForkInfoRequest,
) -> Result<HardForkInfoResponse, Error> {
    let hard_fork = if request.version > 0 {
        HardFork::from_version(request.version)?
    } else {
        blockchain_context::context(&mut state.blockchain_context)
            .await?
            .unchecked_blockchain_context()
            .current_hf
    };

    let info = blockchain_context::hard_fork_info(&mut state.blockchain_context, hard_fork).await?;

    Ok(HardForkInfoResponse {
        base: AccessResponseBase::OK,
        earliest_height: info.earliest_height,
        enabled: info.enabled,
        state: info.state,
        threshold: info.threshold,
        version: info.version,
        votes: info.votes,
        voting: info.voting,
        window: info.window,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2832-L2878>
async fn set_bans(
    state: CupratedRpcHandler,
    request: SetBansRequest,
) -> Result<SetBansResponse, Error> {
    for peer in request.bans {
        let address = todo!();

        let ban = if peer.ban {
            Some(Duration::from_secs(peer.seconds.into()))
        } else {
            None
        };

        let set_ban = cuprate_p2p_core::types::SetBan { address, ban };

        address_book::set_ban::<ClearNet>(&mut DummyAddressBook, set_ban).await?;
    }

    Ok(SetBansResponse {
        base: ResponseBase::OK,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2768-L2801>
async fn get_bans(
    state: CupratedRpcHandler,
    request: GetBansRequest,
) -> Result<GetBansResponse, Error> {
    let now = Instant::now();

    let bans = address_book::get_bans::<ClearNet>(&mut DummyAddressBook)
        .await?
        .into_iter()
        .map(|ban| {
            let seconds = if let Some(instant) = ban.unban_instant {
                instant
                    .checked_duration_since(now)
                    .unwrap_or_default()
                    .as_secs()
                    .try_into()
                    .unwrap_or(0)
            } else {
                0
            };

            GetBan {
                host: ban.address.to_string(),
                ip: todo!(),
                seconds,
            }
        })
        .collect();

    Ok(GetBansResponse {
        base: ResponseBase::OK,
        bans,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2803-L2830>
async fn banned(
    state: CupratedRpcHandler,
    request: BannedRequest,
) -> Result<BannedResponse, Error> {
    let peer = todo!("create Z::Addr from request.address");
    let ban = address_book::get_ban::<ClearNet>(&mut DummyAddressBook, peer).await?;

    let (banned, seconds) = if let Some(instant) = ban {
        let seconds = instant
            .checked_duration_since(Instant::now())
            .unwrap_or_default()
            .as_secs()
            .try_into()
            .unwrap_or(0);

        (true, seconds)
    } else {
        (false, 0)
    };

    Ok(BannedResponse {
        banned,
        seconds,
        status: Status::Ok,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2880-L2932>
async fn flush_transaction_pool(
    mut state: CupratedRpcHandler,
    request: FlushTransactionPoolRequest,
) -> Result<FlushTransactionPoolResponse, Error> {
    let tx_hashes = request
        .txids
        .into_iter()
        .map(helper::hex_to_hash)
        .collect::<Result<Vec<[u8; 32]>, _>>()?;

    txpool::flush(&mut state.txpool_manager, tx_hashes).await?;

    Ok(FlushTransactionPoolResponse { status: Status::Ok })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2934-L2979>
async fn get_output_histogram(
    mut state: CupratedRpcHandler,
    request: GetOutputHistogramRequest,
) -> Result<GetOutputHistogramResponse, Error> {
    let input = cuprate_types::OutputHistogramInput {
        amounts: request.amounts,
        min_count: request.min_count,
        max_count: request.max_count,
        unlocked: request.unlocked,
        recent_cutoff: request.recent_cutoff,
    };

    let histogram = blockchain::output_histogram(&mut state.blockchain_read, input)
        .await?
        .into_iter()
        .map(|entry| HistogramEntry {
            amount: entry.amount,
            total_instances: entry.total_instances,
            unlocked_instances: entry.unlocked_instances,
            recent_instances: entry.recent_instances,
        })
        .collect();

    Ok(GetOutputHistogramResponse {
        base: AccessResponseBase::OK,
        histogram,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2998-L3013>
async fn get_coinbase_tx_sum(
    mut state: CupratedRpcHandler,
    request: GetCoinbaseTxSumRequest,
) -> Result<GetCoinbaseTxSumResponse, Error> {
    let sum =
        blockchain::coinbase_tx_sum(&mut state.blockchain_read, request.height, request.count)
            .await?;

    // Formats `u128` as hexadecimal strings.
    let wide_emission_amount = format!("{:#x}", sum.fee_amount);
    let wide_fee_amount = format!("{:#x}", sum.emission_amount);

    let (emission_amount, emission_amount_top64) =
        split_u128_into_low_high_bits(sum.emission_amount);
    let (fee_amount, fee_amount_top64) = split_u128_into_low_high_bits(sum.fee_amount);

    Ok(GetCoinbaseTxSumResponse {
        base: AccessResponseBase::OK,
        emission_amount,
        emission_amount_top64,
        fee_amount,
        fee_amount_top64,
        wide_emission_amount,
        wide_fee_amount,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2981-L2996>
async fn get_version(
    mut state: CupratedRpcHandler,
    request: GetVersionRequest,
) -> Result<GetVersionResponse, Error> {
    let current_height = helper::top_height(&mut state).await?.0;
    let target_height = blockchain_manager::target_height(&mut state.blockchain_manager).await?;

    let hard_forks = blockchain::hard_forks(&mut state.blockchain_read)
        .await?
        .into_iter()
        .map(|(height, hf)| HardforkEntry {
            height: usize_to_u64(height),
            hf_version: hf.as_u8(),
        })
        .collect();

    Ok(GetVersionResponse {
        base: ResponseBase::OK,
        version: CORE_RPC_VERSION,
        release: RELEASE,
        current_height,
        target_height,
        hard_forks,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3015-L3031>
async fn get_fee_estimate(
    mut state: CupratedRpcHandler,
    request: GetFeeEstimateRequest,
) -> Result<GetFeeEstimateResponse, Error> {
    let estimate =
        blockchain_context::fee_estimate(&mut state.blockchain_context, request.grace_blocks)
            .await?;

    Ok(GetFeeEstimateResponse {
        base: AccessResponseBase::OK,
        fee: estimate.fee,
        fees: estimate.fees,
        quantization_mask: estimate.quantization_mask,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3033-L3064>
async fn get_alternate_chains(
    mut state: CupratedRpcHandler,
    request: GetAlternateChainsRequest,
) -> Result<GetAlternateChainsResponse, Error> {
    let chains = blockchain::alt_chains(&mut state.blockchain_read)
        .await?
        .into_iter()
        .map(|info| {
            let block_hashes = info.block_hashes.into_iter().map(hex::encode).collect();
            let (difficulty, difficulty_top64) = split_u128_into_low_high_bits(info.difficulty);

            ChainInfo {
                block_hash: hex::encode(info.block_hash),
                block_hashes,
                difficulty,
                difficulty_top64,
                height: info.height,
                length: info.length,
                main_chain_parent_block: hex::encode(info.main_chain_parent_block),
                wide_difficulty: hex::encode(u128::to_ne_bytes(info.difficulty)),
            }
        })
        .collect();

    Ok(GetAlternateChainsResponse {
        base: ResponseBase::OK,
        chains,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3254-L3304>
async fn relay_tx(
    mut state: CupratedRpcHandler,
    request: RelayTxRequest,
) -> Result<RelayTxResponse, Error> {
    let tx_hashes = request
        .txids
        .into_iter()
        .map(helper::hex_to_hash)
        .collect::<Result<Vec<[u8; 32]>, _>>()?;

    txpool::relay(&mut state.txpool_manager, tx_hashes).await?;

    Ok(RelayTxResponse { status: Status::Ok })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3306-L3330>
async fn sync_info(
    mut state: CupratedRpcHandler,
    request: SyncInfoRequest,
) -> Result<SyncInfoResponse, Error> {
    let height = usize_to_u64(
        blockchain_context::context(&mut state.blockchain_context)
            .await?
            .unchecked_blockchain_context()
            .chain_height,
    );

    let target_height = blockchain_manager::target_height(&mut state.blockchain_manager).await?;

    let peers = address_book::connection_info::<ClearNet>(&mut DummyAddressBook)
        .await?
        .into_iter()
        .map(|info| SyncInfoPeer { info })
        .collect();

    let next_needed_pruning_seed =
        address_book::next_needed_pruning_seed::<ClearNet>(&mut DummyAddressBook)
            .await?
            .compress();
    let overview = blockchain_manager::overview(&mut state.blockchain_manager, height).await?;
    let spans = address_book::spans::<ClearNet>(&mut DummyAddressBook).await?;

    Ok(SyncInfoResponse {
        base: AccessResponseBase::OK,
        height,
        next_needed_pruning_seed,
        overview,
        peers,
        spans,
        target_height,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3332-L3350>
async fn get_transaction_pool_backlog(
    mut state: CupratedRpcHandler,
    request: GetTransactionPoolBacklogRequest,
) -> Result<GetTransactionPoolBacklogResponse, Error> {
    let backlog = txpool::backlog(&mut state.txpool_read)
        .await?
        .into_iter()
        .map(|entry| TxBacklogEntry {
            weight: entry.weight,
            fee: entry.fee,
            time_in_pool: entry.time_in_pool.as_secs(),
        })
        .collect();

    Ok(GetTransactionPoolBacklogResponse {
        base: ResponseBase::OK,
        backlog,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1998-L2033>
async fn get_miner_data(
    mut state: CupratedRpcHandler,
    request: GetMinerDataRequest,
) -> Result<GetMinerDataResponse, Error> {
    let context = blockchain_context::context(&mut state.blockchain_context).await?;
    let context = context.unchecked_blockchain_context();
    let major_version = context.current_hf.as_u8();
    let height = usize_to_u64(context.chain_height);
    let prev_id = hex::encode(context.top_hash);
    let seed_hash = todo!();
    let difficulty = format!("{:#x}", context.next_difficulty);
    let median_weight = usize_to_u64(context.median_weight_for_block_reward);
    let already_generated_coins = context.already_generated_coins;
    let tx_backlog = txpool::block_template_backlog(&mut state.txpool_read)
        .await?
        .into_iter()
        .map(|entry| GetMinerDataTxBacklogEntry {
            id: hex::encode(entry.id),
            weight: entry.weight,
            fee: entry.fee,
        })
        .collect();

    Ok(GetMinerDataResponse {
        base: ResponseBase::OK,
        major_version,
        height,
        prev_id,
        seed_hash,
        difficulty,
        median_weight,
        already_generated_coins,
        tx_backlog,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3453-L3476>
async fn prune_blockchain(
    mut state: CupratedRpcHandler,
    request: PruneBlockchainRequest,
) -> Result<PruneBlockchainResponse, Error> {
    let pruned = blockchain_manager::pruned(&mut state.blockchain_manager).await?;
    let pruning_seed = blockchain_manager::prune(&mut state.blockchain_manager)
        .await?
        .compress();

    Ok(PruneBlockchainResponse {
        base: ResponseBase::OK,
        pruned,
        pruning_seed,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2035-L2070>
async fn calc_pow(
    mut state: CupratedRpcHandler,
    mut request: CalcPowRequest,
) -> Result<CalcPowResponse, Error> {
    let hardfork = HardFork::from_version(request.major_version)?;
    let mut block_blob: Vec<u8> = hex::decode(request.block_blob)?;
    let block = Block::read(&mut block_blob.as_slice())?;
    let seed_hash = helper::hex_to_hash(request.seed_hash)?;

    let pow_hash = blockchain_manager::calculate_pow(
        &mut state.blockchain_manager,
        hardfork,
        request.height,
        block,
        seed_hash,
    )
    .await?;

    let hex = hex::encode(pow_hash);

    Ok(CalcPowResponse { pow_hash: hex })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3542-L3551>
async fn flush_cache(
    state: CupratedRpcHandler,
    request: FlushCacheRequest,
) -> Result<FlushCacheResponse, Error> {
    // TODO: cuprated doesn't need this call; decide behavior.

    Ok(FlushCacheResponse {
        base: ResponseBase::OK,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2072-L2207>
async fn add_aux_pow(
    mut state: CupratedRpcHandler,
    request: AddAuxPowRequest,
) -> Result<AddAuxPowResponse, Error> {
    let hex = hex::decode(request.blocktemplate_blob)?;
    let block_template = Block::read(&mut hex.as_slice())?;

    let aux_pow = request
        .aux_pow
        .into_iter()
        .map(|aux| {
            let id = helper::hex_to_hash(aux.id)?;
            let hash = helper::hex_to_hash(aux.hash)?;
            Ok(cuprate_types::AuxPow { id, hash })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let resp =
        blockchain_manager::add_aux_pow(&mut state.blockchain_manager, block_template, aux_pow)
            .await?;

    let blocktemplate_blob = hex::encode(resp.blocktemplate_blob);
    let blockhashing_blob = hex::encode(resp.blockhashing_blob);
    let merkle_root = hex::encode(resp.merkle_root);
    let aux_pow = resp
        .aux_pow
        .into_iter()
        .map(|aux| AuxPow {
            id: hex::encode(aux.id),
            hash: hex::encode(aux.hash),
        })
        .collect::<Vec<AuxPow>>();

    Ok(AddAuxPowResponse {
        base: ResponseBase::OK,
        blocktemplate_blob,
        blockhashing_blob,
        merkle_root,
        merkle_tree_depth: resp.merkle_tree_depth,
        aux_pow,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3553-L3627>
async fn get_tx_ids_loose(
    state: CupratedRpcHandler,
    request: GetTxIdsLooseRequest,
) -> Result<GetTxIdsLooseResponse, Error> {
    // TODO: this RPC call is not yet in the v0.18 branch.
    return Err(anyhow!("not implemented"));

    Ok(GetTxIdsLooseResponse {
        base: ResponseBase::OK,
        txids: todo!(),
    })
}
