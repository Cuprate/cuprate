#![cfg(feature = "binaries")]

use cuprate_common::Network;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use tower::{Service, ServiceExt};
use tracing::instrument;
use tracing::level_filters::LevelFilter;



use monero_consensus::rpc::{init_rpc_load_balancer, MAX_BLOCKS_IN_RANGE};
use monero_consensus::{
    state::{Config, State},
    ConsensusError, Database, DatabaseRequest, DatabaseResponse,
};

/// A cache which can keep chain state while scanning.
///
/// Because we are using a RPC interface with node we need to keep track
/// of certain data that node doesn't hold like the number of outputs at
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

#[instrument(skip_all, level = "info")]
async fn scan_chain<D: Database + Clone>(
    cache: ScanningCache,
    network: Network,
    mut database: D,
) -> Result<(), ConsensusError> {
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

    let _state = State::init_at_chain_height(config, cache.height, database.clone()).await?;

    tracing::info!("Initialised state, begging scan");

    for height in (cache.height..chain_height).step_by(MAX_BLOCKS_IN_RANGE as usize) {
        let DatabaseResponse::BlockBatchInRange(_blocks) = database
            .ready()
            .await?
            .call(DatabaseRequest::BlockBatchInRange(
                height..(height + MAX_BLOCKS_IN_RANGE).max(chain_height),
            ))
            .await?
        else {
            panic!("Database sent incorrect response!");
        };
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
        "http://node.community.rino.io:18081".to_string(),
        "http://nodes.hashvault.pro:18081".to_string(),
        "http://node.moneroworld.com:18089".to_string(),
        "http://node.c3pool.com:18081".to_string(),
        //
        "http://xmr-node.cakewallet.com:18081".to_string(),
        "http://node.sethforprivacy.com".to_string(),
        "http://nodex.monerujo.io:18081".to_string(),
        "http://node.community.rino.io:18081".to_string(),
        "http://nodes.hashvault.pro:18081".to_string(),
        "http://node.moneroworld.com:18089".to_string(),
        "http://node.c3pool.com:18081".to_string(),
    ];

    let rpc = init_rpc_load_balancer(urls);

    let network = Network::Mainnet;
    let cache = ScanningCache::default();

    scan_chain(cache, network, rpc).await.unwrap();
}
