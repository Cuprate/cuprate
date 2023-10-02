#![cfg(feature = "binaries")]

use tower::ServiceExt;
use tracing::level_filters::LevelFilter;

use monero_consensus::block::{pow::difficulty::DifficultyCache, weight::BlockWeightsCache};
use monero_consensus::hardforks::HardFork;
use monero_consensus::rpc::init_rpc_load_balancer;
use monero_consensus::{DatabaseRequest, DatabaseResponse};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
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

    let mut rpc = init_rpc_load_balancer(urls);

    let mut difficulty = DifficultyCache::init_from_chain_height(2985610, rpc.clone())
        .await
        .unwrap();
    /*
    let DatabaseResponse::BlockWeights(weights) = rpc
        .oneshot(DatabaseRequest::BlockWeights(2985610.into()))
        .await
        .unwrap()
    else {
        panic!()
    };

    assert_eq!(
        weights.long_term_weight,
        difficulty.next_block_long_term_weight(&HardFork::V16, weights.block_weight)
    );

     */
    println!("{:?}", difficulty.next_difficulty(&HardFork::V16)); //774466376

    //let _hfs = HardForks::init_at_chain_height(HardForkConfig::default(), 1009827, rpc.clone())
    //    .await
    //    .unwrap();
}
