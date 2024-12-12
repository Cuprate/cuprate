use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

mod rpc;

pub static TESTED_BLOCK_COUNT: AtomicUsize = AtomicUsize::new(0);

#[tokio::main]
async fn main() {
    let now = Instant::now();

    let rpc_node_url = if let Ok(url) = std::env::var("RPC_NODE_URL") {
        url
    } else {
        "http://127.0.0.1:18081/json_rpc".to_string()
    };
    println!("rpc_node_url: {rpc_node_url}");

    let rpc_client = rpc::RpcClient::new(rpc_node_url).await;

    tokio::join!(
        rpc_client.cryptonight_v0(),
        rpc_client.cryptonight_v1(),
        rpc_client.cryptonight_v2(),
        rpc_client.cryptonight_r(),
        rpc_client.randomx(),
    );

    println!("finished all PoW, took: {}s", now.elapsed().as_secs());
}
