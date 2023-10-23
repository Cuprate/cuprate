#![cfg(feature = "binaries")]

use futures::Sink;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Read;
use std::ops::Range;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use rayon::prelude::*;
use tower::{Service, ServiceExt};
use tracing::instrument;
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
    network: Network,
    rpc_config: Arc<RwLock<RpcConfig>>,
    mut database: D,
) -> Result<(), tower::BoxError>
where
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    tracing::info!("Beginning chain scan");

    let chain_height = 3_000_000;

    tracing::info!("scanning to chain height: {}", chain_height);

    let config = ContextConfig::main_net();

    let (mut block_verifier, _, mut context_updater) =
        initialize_verifier(database.clone(), config).await?;

    let batch_size = rpc_config.read().unwrap().block_batch_size();
    let start_height = cache.read().unwrap().height;

    tracing::info!(
        "Initialised verifier, begging scan from {} to {}",
        start_height,
        chain_height
    );

    let mut next_fut = tokio::spawn(call_batch(
        start_height..(start_height + batch_size).min(chain_height),
        database.clone(),
    ));

    let mut current_height = start_height;
    let mut next_batch_start_height = start_height + batch_size;

    let mut time_to_verify_last_batch: u128 = 0;

    let mut batches_till_check_batch_size: u64 = 2;

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

        let (DatabaseResponse::BlockBatchInRange(blocks), time_to_retrieve_batch) =
            current_fut.await??
        else {
            panic!("Database sent incorrect response!");
        };

        let time_to_verify_batch = std::time::Instant::now();

        let time_to_retrieve_batch = time_to_retrieve_batch.as_millis();
        /*
               if time_to_retrieve_batch > time_to_verify_last_batch + 2000
                   && batches_till_check_batch_size == 0
               {
                   batches_till_check_batch_size = 3;

                   let mut conf = rpc_config.write().unwrap();
                   tracing::info!(
                       "Decreasing batch size time to verify last batch: {}, time_to_retrieve_batch: {}",
                       time_to_verify_last_batch,
                       time_to_retrieve_batch
                   );
                   conf.max_blocks_per_node = (conf.max_blocks_per_node
                       * time_to_verify_last_batch as u64
                       / (time_to_retrieve_batch as u64))
                       .max(10_u64)
                       .min(MAX_BLOCKS_IN_RANGE);
                   tracing::info!("Decreasing batch size to: {}", conf.max_blocks_per_node);
               } else if time_to_retrieve_batch + 2000 < time_to_verify_last_batch
                   && batches_till_check_batch_size == 0
               {
                   batches_till_check_batch_size = 3;

                   let mut conf = rpc_config.write().unwrap();
                   tracing::info!(
                       "Increasing batch size time to verify last batch: {}, time_to_retrieve_batch: {}",
                       time_to_verify_last_batch,
                       time_to_retrieve_batch
                   );
                   conf.max_blocks_per_node = (conf.max_blocks_per_node
                       * (time_to_verify_last_batch as u64)
                       / time_to_retrieve_batch.max(1) as u64)
                       .max(30_u64)
                       .min(MAX_BLOCKS_IN_RANGE);
                   tracing::info!("Increasing batch size to: {}", conf.max_blocks_per_node);
               } else {
                   batches_till_check_batch_size = batches_till_check_batch_size.saturating_sub(1);
               }

        */

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
                })
                .await?;

            tracing::info!("Verified block: {}", current_height);

            current_height += 1;
            next_batch_start_height += 1;
        }

        time_to_verify_last_batch = time_to_verify_batch.elapsed().as_millis();
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();

    let network = Network::Mainnet;

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

    let cache = Arc::new(RwLock::new(ScanningCache::default()));

    let mut cache_write = cache.write().unwrap();

    if cache_write.height == 0 {
        let genesis = monero_consensus::genesis::generate_genesis_block(&network);

        let total_outs = genesis
            .miner_tx
            .prefix
            .outputs
            .iter()
            .map(|out| out.amount.unwrap_or(0))
            .sum::<u64>();

        cache_write.add_new_block_data(total_outs, &genesis.miner_tx, &[]);
    }
    drop(cache_write);

    let rpc = init_rpc_load_balancer(urls, cache.clone(), rpc_config.clone());

    scan_chain(cache, network, rpc_config, rpc).await.unwrap();
}
