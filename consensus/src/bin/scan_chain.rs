#![cfg(feature = "binaries")]

use std::path::Path;
use std::{
    io::Read,
    ops::Range,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use monero_serai::{block::Block, transaction::Transaction};
use tower::{Service, ServiceExt};
use tracing::level_filters::LevelFilter;

use cuprate_common::Network;

use monero_consensus::{
    context::{ContextConfig, UpdateBlockchainCacheRequest},
    initialize_verifier,
    rpc::{cache::ScanningCache, init_rpc_load_balancer, RpcConfig},
    transactions::VerifyTxRequest,
    ConsensusError, Database, DatabaseRequest, DatabaseResponse, HardFork,
    VerifiedBlockInformation, VerifyBlockRequest, VerifyTxResponse,
};

const MAX_BLOCKS_IN_RANGE: u64 = 500;
const MAX_BLOCKS_HEADERS_IN_RANGE: u64 = 250;

/// Calls for a batch of blocks, returning the response and the time it took.
async fn call_batch<D: Database>(
    range: Range<u64>,
    database: D,
) -> Result<DatabaseResponse, tower::BoxError> {
    database
        .oneshot(DatabaseRequest::BlockBatchInRange(range))
        .await
}

fn simple_get_hf(height: u64) -> HardFork {
    match height {
        0..=1009826 => HardFork::V1,
        1009827..=1141316 => HardFork::V2,
        1141317..=1220515 => HardFork::V3,
        _ => todo!("rules past v3"),
    }
}

async fn update_cache_and_context<Ctx>(
    cache: &RwLock<ScanningCache>,
    context_updater: &mut Ctx,
    verified_block_info: VerifiedBlockInformation,
) -> Result<(), tower::BoxError>
where
    Ctx: tower::Service<UpdateBlockchainCacheRequest, Response = (), Error = tower::BoxError>,
{
    // add the new block to the cache
    cache.write().unwrap().add_new_block_data(
        verified_block_info.generated_coins,
        &verified_block_info.block.miner_tx,
        &verified_block_info.txs,
    );
    // update the chain context svc with the new block
    context_updater
        .ready()
        .await?
        .call(UpdateBlockchainCacheRequest {
            new_top_hash: verified_block_info.block_hash,
            height: verified_block_info.height,
            timestamp: verified_block_info.block.header.timestamp,
            weight: verified_block_info.weight,
            long_term_weight: verified_block_info.long_term_weight,
            vote: verified_block_info.hf_vote,
            generated_coins: verified_block_info.generated_coins,
            cumulative_difficulty: verified_block_info.cumulative_difficulty,
        })
        .await?;

    Ok(())
}

/// Batches all transactions together when getting outs
///
/// TODO: reduce the amount of parameters of this function
async fn batch_txs_verify_blocks<Tx, Blk, Ctx>(
    cache: &RwLock<ScanningCache>,
    save_file: &Path,
    txs: Vec<Vec<Transaction>>,
    blocks: Vec<Block>,
    tx_verifier: &mut Tx,
    block_verifier: &mut Blk,
    context_updater: &mut Ctx,
    current_height: u64,
    hf: HardFork,
) -> Result<(), tower::BoxError>
where
    Blk: tower::Service<
        VerifyBlockRequest,
        Response = VerifiedBlockInformation,
        Error = ConsensusError,
    >,
    Tx: tower::Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ConsensusError>,
    Ctx: tower::Service<UpdateBlockchainCacheRequest, Response = (), Error = tower::BoxError>,
{
    // This is an optimisation, we batch ALL the transactions together to get their outputs, saving a
    // massive amount of time at the cost of inaccurate data, specifically the only thing that's inaccurate
    // is the amount of outputs at a certain time and as this would be lower (so more strict) than the true value
    // this will fail when this is an issue.
    let mut txs_per_block = [0; (MAX_BLOCKS_IN_RANGE * 3) as usize];
    let txs = txs
        .into_iter()
        .enumerate()
        .flat_map(|(block_id, block_batch_txs)| {
            // block id is just this blocks position in the batch.
            txs_per_block[block_id] = block_batch_txs.len();
            block_batch_txs
        })
        .collect();

    let VerifyTxResponse::BatchSetupOk(txs) = tx_verifier
        .ready()
        .await?
        .call(VerifyTxRequest::BatchSetup { txs, hf })
        .await?
    else {
        panic!("tx verifier returned incorrect response");
    };

    let mut done_txs = 0;
    for (block_id, block) in blocks.into_iter().enumerate() {
        // block id is just this blocks position in the batch.
        let txs = &txs[done_txs..done_txs + txs_per_block[block_id]];
        done_txs += txs_per_block[block_id];

        let verified_block_info: VerifiedBlockInformation = block_verifier
            .ready()
            .await?
            .call(VerifyBlockRequest::MainChain(block, txs.into()))
            .await?;

        tracing::info!(
            "verified block: {}",
            current_height + u64::try_from(block_id).unwrap()
        );

        update_cache_and_context(cache, context_updater, verified_block_info).await?;

        if current_height + u64::try_from(block_id).unwrap() % 25000 == 0 {
            tracing::info!("Saving cache to: {}", save_file.display());
            cache.read().unwrap().save(save_file)?;
        }
    }

    Ok(())
}

/// Batches only transactions per block together when getting outs
///
/// TODO: reduce the amount of parameters of this function
async fn verify_blocks<Blk, Ctx>(
    cache: &RwLock<ScanningCache>,
    save_file: &Path,
    txs: Vec<Vec<Transaction>>,
    blocks: Vec<Block>,
    block_verifier: &mut Blk,
    context_updater: &mut Ctx,
    current_height: u64,
) -> Result<(), tower::BoxError>
where
    Blk: tower::Service<
        VerifyBlockRequest,
        Response = VerifiedBlockInformation,
        Error = ConsensusError,
    >,
    Ctx: tower::Service<UpdateBlockchainCacheRequest, Response = (), Error = tower::BoxError>,
{
    for (block_id, (block, txs)) in blocks.into_iter().zip(txs.into_iter()).enumerate() {
        let verified_block_info: VerifiedBlockInformation = block_verifier
            .ready()
            .await?
            .call(VerifyBlockRequest::MainChainBatchSetupVerify(block, txs))
            .await?;

        tracing::info!(
            "verified block: {}",
            current_height + u64::try_from(block_id).unwrap()
        );

        update_cache_and_context(cache, context_updater, verified_block_info).await?;

        if current_height + u64::try_from(block_id).unwrap() % 25000 == 0 {
            tracing::info!("Saving cache to: {}", save_file.display());
            cache.read().unwrap().save(save_file)?;
        }
    }

    Ok(())
}

async fn scan_chain<D>(
    cache: Arc<RwLock<ScanningCache>>,
    save_file: PathBuf,
    rpc_config: Arc<RwLock<RpcConfig>>,
    mut database: D,
) -> Result<(), tower::BoxError>
where
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    tracing::info!("Beginning chain scan");

    // TODO: when we implement all rules use the RPCs chain height, for now we don't check v2 txs.
    let chain_height = 3_000_000;

    tracing::info!("scanning to chain height: {}", chain_height);

    let config = ContextConfig::main_net();

    let (mut block_verifier, mut transaction_verifier, mut context_updater) =
        initialize_verifier(database.clone(), config).await?;

    let batch_size = rpc_config.read().unwrap().block_batch_size();
    let start_height = cache.read().unwrap().height;

    tracing::info!(
        "Initialised verifier, beginning scan from {} to {}",
        start_height,
        chain_height
    );

    let mut next_fut = tokio::spawn(call_batch(
        start_height..(start_height + batch_size).min(chain_height),
        database.clone(),
    ));

    let mut current_height = start_height;
    let mut next_batch_start_height = start_height + batch_size;

    while next_batch_start_height < chain_height {
        // TODO: utilize dynamic batch sizes
        let next_batch_size = rpc_config.read().unwrap().block_batch_size();

        // Call the next batch while we handle this batch.
        let current_fut = std::mem::replace(
            &mut next_fut,
            tokio::spawn(call_batch(
                next_batch_start_height
                    ..(next_batch_start_height + next_batch_size).min(chain_height),
                database.clone(),
            )),
        );

        let DatabaseResponse::BlockBatchInRange(blocks) = current_fut.await?? else {
            panic!("Database sent incorrect response!");
        };

        tracing::info!(
            "Handling batch: {:?}, chain height: {}",
            current_height..(current_height + blocks.len() as u64),
            chain_height
        );

        let (blocks, txs): (Vec<_>, Vec<_>) = blocks.into_iter().unzip();
        let batch_len = u64::try_from(blocks.len()).unwrap();

        let hf_start_batch = simple_get_hf(current_height);
        let hf_end_batch = simple_get_hf(current_height + batch_len);

        if hf_start_batch == hf_end_batch {
            // we can only batch transactions on the same hard fork
            batch_txs_verify_blocks(
                &cache,
                &save_file,
                txs,
                blocks,
                &mut transaction_verifier,
                &mut block_verifier,
                &mut context_updater,
                current_height,
                hf_start_batch,
            )
            .await?;
        } else {
            tracing::warn!(
                "Hard fork during batch, getting outputs per block this will take a while!"
            );
            verify_blocks(
                &cache,
                &save_file,
                txs,
                blocks,
                &mut block_verifier,
                &mut context_updater,
                current_height,
            )
            .await?;
        }
        current_height += batch_len;
        next_batch_start_height += batch_len;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // TODO: take this in as config options:
    // - nodes to connect to
    // - block batch size (not header)
    // - network
    // - tracing level

    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();

    let network = Network::Mainnet;

    let mut file_for_cache = dirs::cache_dir().unwrap();
    file_for_cache.push("cuprate_rpc_scanning_cache.bin");

    let urls = vec![
        "http://xmr-node.cakewallet.com:18081".to_string(),
        "http://node.sethforprivacy.com".to_string(),
        "http://nodex.monerujo.io:18081".to_string(),
        "http://nodes.hashvault.pro:18081".to_string(),
        "http://node.c3pool.com:18081".to_string(),
        "http://node.trocador.app:18089".to_string(),
        "http://xmr.lukas.services:18089".to_string(),
        "http://xmr-node-eu.cakewallet.com:18081".to_string(),
        "http://38.105.209.54:18089".to_string(),
        "http://68.118.241.70:18089".to_string(),
        "http://145.239.97.211:18089".to_string(),
        //
        "http://xmr-node.cakewallet.com:18081".to_string(),
        "http://node.sethforprivacy.com".to_string(),
        "http://nodex.monerujo.io:18081".to_string(),
        "http://nodes.hashvault.pro:18081".to_string(),
        "http://node.c3pool.com:18081".to_string(),
        "http://node.trocador.app:18089".to_string(),
        "http://xmr.lukas.services:18089".to_string(),
        "http://xmr-node-eu.cakewallet.com:18081".to_string(),
        "http://38.105.209.54:18089".to_string(),
        "http://68.118.241.70:18089".to_string(),
        "http://145.239.97.211:18089".to_string(),
    ];

    let rpc_config = RpcConfig::new(MAX_BLOCKS_IN_RANGE, MAX_BLOCKS_HEADERS_IN_RANGE);
    let rpc_config = Arc::new(RwLock::new(rpc_config));

    let cache = match ScanningCache::load(&file_for_cache) {
        Ok(cache) => {
            tracing::info!("Reloaded from cache, chain height: {}", cache.height);
            Arc::new(RwLock::new(cache))
        }
        Err(_) => {
            tracing::info!("Couldn't load from cache starting from scratch");
            let mut cache = ScanningCache::default();
            let genesis = monero_consensus::genesis::generate_genesis_block(&network);

            let total_outs = genesis
                .miner_tx
                .prefix
                .outputs
                .iter()
                .map(|out| out.amount.unwrap_or(0))
                .sum::<u64>();

            cache.add_new_block_data(total_outs, &genesis.miner_tx, &[]);
            Arc::new(RwLock::new(cache))
        }
    };

    let rpc = init_rpc_load_balancer(urls, cache.clone(), rpc_config.clone());

    scan_chain(cache, file_for_cache, rpc_config, rpc)
        .await
        .unwrap();
}
