mod cryptonight;
mod randomx;
mod rpc;
mod verify;

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use crate::rpc::GetBlockResponse;

pub const RANDOMX_START_HEIGHT: u64 = 1978433;
pub static TESTED_BLOCK_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct VerifyData {
    pub get_block_response: GetBlockResponse,
    pub height: u64,
    pub seed_height: u64,
    pub seed_hash: [u8; 32],
}

#[tokio::main]
async fn main() {
    let now = Instant::now();

    let rpc_url = if let Ok(url) = std::env::var("RPC_URL") {
        println!("RPC_URL (found): {url}");
        url
    } else {
        let rpc_url = "http://127.0.0.1:18081".to_string();
        println!("RPC_URL (off, using default): {rpc_url}");
        rpc_url
    };
    if std::env::var("VERBOSE").is_ok() {
        println!("VERBOSE: true");
    } else {
        println!("VERBOSE: false");
    }

    let client = rpc::RpcClient::new(rpc_url).await;
    let top_height = client.top_height;
    println!("top_height: {top_height}");

    let threads = if let Ok(Ok(c)) = std::env::var("THREADS").map(|s| s.parse()) {
        println!("THREADS (found): {c}");
        c
    } else {
        let c = std::thread::available_parallelism().unwrap().get();
        println!("THREADS (off): {c}");
        c
    };

    println!();

    // Test RandomX.
    let (tx, rx) = crossbeam::channel::unbounded();
    verify::spawn_verify_pool(threads, top_height, rx);
    client.test(top_height, tx).await;

    // Wait for other threads to finish.
    loop {
        let count = TESTED_BLOCK_COUNT.load(Ordering::Acquire);

        if top_height == count {
            println!("finished, took {}s", now.elapsed().as_secs());
            std::process::exit(0);
        }

        std::thread::sleep(Duration::from_secs(1));
    }
}
