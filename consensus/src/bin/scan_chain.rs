#![cfg(feature = "binaries")]

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, RwLock};

use tower::ServiceExt;
use tracing::instrument;
use tracing::level_filters::LevelFilter;

use cuprate_common::Network;

use monero_consensus::hardforks::HardFork;
use monero_consensus::rpc::{init_rpc_load_balancer, RpcConfig};
use monero_consensus::{
    verifier::{Config, Verifier},
    Database, DatabaseRequest, DatabaseResponse,
};

const INITIAL_MAX_BLOCKS_IN_RANGE: u64 = 250;
const MAX_BLOCKS_IN_RANGE: u64 = 1000;
const INITIAL_MAX_BLOCKS_HEADERS_IN_RANGE: u64 = 250;

/// A cache which can keep chain state while scanning.
///
/// Because we are using a RPC interface with a node we need to keep track
/// of certain data that the node doesn't hold like the number of outputs at
/// a certain time.
#[derive(Debug, Clone)]
struct ScanningCache {
    network: Network,
    numb_outs: HashMap<u64, u64>,
    /// The height of the *next* block to scan.
    height: u64,
}

impl Default for ScanningCache {
    fn default() -> Self {
        ScanningCache {
            network: Default::default(),
            numb_outs: Default::default(),
            height: 1,
        }
    }
}

impl ScanningCache {
    fn total_outs(&self) -> u64 {
        self.numb_outs.values().sum()
    }

    fn numb_outs(&self, amount: u64) -> u64 {
        *self.numb_outs.get(&amount).unwrap_or(&0)
    }

    fn add_outs(&mut self, amount: u64, count: u64) {
        if let Some(numb_outs) = self.numb_outs.get_mut(&amount) {
            *numb_outs += count;
        } else {
            self.numb_outs.insert(amount, count);
        }
    }
}

impl Display for ScanningCache {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let rct_outs = self.numb_outs(0);
        let total_outs = self.total_outs();

        f.debug_struct("Cache")
            .field("next_block", &self.height)
            .field("rct_outs", &rct_outs)
            .field("total_outs", &total_outs)
            .finish()
    }
}
async fn scan_chain<D: Database + Clone + Send + 'static>(
    cache: ScanningCache,
    network: Network,
    rpc_config: Arc<RwLock<RpcConfig>>,
    mut database: D,
) -> Result<(), tower::BoxError>
where
    D::Future: Send + 'static,
{
    tracing::info!("Beginning chain scan, {}", &cache);

    let DatabaseResponse::ChainHeight(chain_height) = database
        .ready()
        .await?
        .call(DatabaseRequest::ChainHeight)
        .await?
    else {
        panic!("Database sent incorrect response!");
    };

    tracing::info!("scanning to chain height: {}", chain_height);

    let config = match network {
        Network::Mainnet => Config::main_net(),
        _ => todo!(),
    };

    //let verifier = Verifier::init_at_chain_height(config, cache.height, database.clone()).await?;

    tracing::info!("Initialised verifier, begging scan");
    let batch_size = rpc_config.read().unwrap().block_batch_size();

    let mut next_fut = tokio::spawn(database.clone().ready().await?.call(
        DatabaseRequest::BlockBatchInRange(
            cache.height..(cache.height + batch_size).min(chain_height),
        ),
    ));

    let mut current_height = cache.height;
    let mut next_batch_start_height = cache.height + batch_size;

    let mut time_to_verify_last_batch: u128 = 0;

    let mut first = true;

    while next_batch_start_height < chain_height {
        let next_batch_size = rpc_config.read().unwrap().block_batch_size();
        let time_to_retrieve_batch = std::time::Instant::now();

        // Call the next batch while we handle this batch.
        let current_fut = std::mem::replace(
            &mut next_fut,
            tokio::spawn(
                database
                    .ready()
                    .await?
                    .call(DatabaseRequest::BlockBatchInRange(
                        next_batch_start_height
                            ..(next_batch_start_height + next_batch_size).min(chain_height),
                    )),
            ),
        );

        let DatabaseResponse::BlockBatchInRange(blocks) = current_fut.await?? else {
            panic!("Database sent incorrect response!");
        };

        let time_to_verify_batch = std::time::Instant::now();

        let time_to_retrieve_batch = time_to_retrieve_batch.elapsed().as_millis();

        if time_to_retrieve_batch > 2000 && !first {
            let mut conf = rpc_config.write().unwrap();
            conf.max_blocks_per_node = (conf.max_blocks_per_node
                * TryInto::<u64>::try_into(
                    time_to_verify_last_batch
                        / (time_to_verify_last_batch + time_to_retrieve_batch),
                )
                .unwrap())
            .max(10_u64)
            .min(MAX_BLOCKS_IN_RANGE);
            tracing::info!("Decreasing batch size to: {}", conf.max_blocks_per_node);
        } else if time_to_retrieve_batch < 100 {
            let mut conf = rpc_config.write().unwrap();
            conf.max_blocks_per_node = (conf.max_blocks_per_node * 2)
                .max(10_u64)
                .min(MAX_BLOCKS_IN_RANGE);
            tracing::info!("Increasing batch size to: {}", conf.max_blocks_per_node);
        }

        first = false;

        tracing::info!(
            "Handling batch: {:?}, chain height: {}",
            current_height..(current_height + blocks.len() as u64),
            chain_height
        );

        for (block, txs) in blocks.into_iter() {
            let pow_hash = monero_consensus::block::pow::calculate_pow_hash(
                &block.serialize_hashable(),
                block.number() as u64,
                &HardFork::V1,
            );

            tracing::info!(
                "Verified block: {}, numb txs: {}",
                current_height,
                txs.len()
            );

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

    let rpc_config = RpcConfig::new(10, INITIAL_MAX_BLOCKS_HEADERS_IN_RANGE);
    let rpc_config = Arc::new(RwLock::new(rpc_config));

    let rpc = init_rpc_load_balancer(urls, rpc_config.clone());

    let network = Network::Mainnet;
    let cache = ScanningCache::default();

    scan_chain(cache, network, rpc_config, rpc).await.unwrap();
}
