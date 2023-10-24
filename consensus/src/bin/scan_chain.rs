#![cfg(feature = "binaries")]

use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tower::{Service, ServiceExt};
use tracing::level_filters::LevelFilter;

use cuprate_common::Network;

use monero_consensus::rpc::{cache::ScanningCache, init_rpc_load_balancer, RpcConfig};
use monero_consensus::{
    context::{ContextConfig, UpdateBlockchainCacheRequest},
    initialize_verifier, Database, DatabaseRequest, DatabaseResponse, VerifiedBlockInformation,
    VerifyBlockRequest,
};

const INITIAL_MAX_BLOCKS_IN_RANGE: u64 = 1000;
const MAX_BLOCKS_IN_RANGE: u64 = 1000;
const INITIAL_MAX_BLOCKS_HEADERS_IN_RANGE: u64 = 250;

/// Calls for a batch of blocks, returning the response and the time it took.
async fn call_batch<D: Database>(
    range: Range<u64>,
    database: D,
) -> Result<(DatabaseResponse, Duration), tower::BoxError> {
    let now = std::time::Instant::now();
    Ok((
        database
            .oneshot(DatabaseRequest::BlockBatchInRange(range))
            .await?,
        now.elapsed(),
    ))
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
    let chain_height = 1288616;

    tracing::info!("scanning to chain height: {}", chain_height);

    let config = ContextConfig::main_net();

    let (mut block_verifier, _, mut context_updater) =
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

        let (DatabaseResponse::BlockBatchInRange(blocks), _) = current_fut.await?? else {
            panic!("Database sent incorrect response!");
        };

        tracing::info!(
            "Handling batch: {:?}, chain height: {}",
            current_height..(current_height + blocks.len() as u64),
            chain_height
        );

        //  let block_len = blocks.len();
        for (block, txs) in blocks {
            let verified_block_info: VerifiedBlockInformation = block_verifier
                .ready()
                .await?
                .call(VerifyBlockRequest::MainChainBatchSetupVerify(block, txs))
                .await?;

            cache.write().unwrap().add_new_block_data(
                verified_block_info.generated_coins,
                &verified_block_info.block.miner_tx,
                &verified_block_info.txs,
            );
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

            tracing::info!("Verified block: {}", current_height);

            current_height += 1;
            next_batch_start_height += 1;

            if current_height % 500 == 0 {
                tracing::info!("Saving cache to: {}", save_file.display());
                cache.write().unwrap().save(&save_file)?;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
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

    let rpc_config = RpcConfig::new(
        INITIAL_MAX_BLOCKS_IN_RANGE,
        INITIAL_MAX_BLOCKS_HEADERS_IN_RANGE,
    );
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
