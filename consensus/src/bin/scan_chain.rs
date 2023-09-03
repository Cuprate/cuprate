#![cfg(feature = "binaries")]

use tower::{Service, ServiceExt};
use tracing::level_filters::LevelFilter;

use monero_consensus::hardforks::{HardFork, HardForkConfig, HardForks};
use monero_consensus::rpc::Rpc;
use monero_consensus::DatabaseRequest;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();

    let mut rpc = Rpc::new_http("http://xmr-node.cakewallet.com:18081".to_string());

    let res = rpc
        .ready()
        .await
        .unwrap()
        .call(DatabaseRequest::ChainHeight)
        .await
        .unwrap();

    println!("{:?}", res);

    let mut hfs = HardForks::init(HardForkConfig::default(), &mut rpc)
        .await
        .unwrap();

    println!("{:?}", hfs);

    hfs.new_block(HardFork::V2, 1009827, &mut rpc).await;
    println!("{:?}", hfs);

    hfs.new_block(HardFork::V2, 1009828, &mut rpc).await;
    println!("{:?}", hfs);
}
