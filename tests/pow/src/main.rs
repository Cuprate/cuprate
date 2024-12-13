mod rpc;

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

pub static TESTED_BLOCK_COUNT: AtomicUsize = AtomicUsize::new(0);

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

    let mut client = rpc::RpcClient::new(rpc_url).await;

    let top_height = if let Ok(Ok(h)) = std::env::var("TOP_HEIGHT").map(|s| s.parse()) {
        client.top_height = h;
        println!("TOP_HEIGHT (found): {h}");
        h
    } else {
        println!("TOP_HEIGHT (off, using latest): {}", client.top_height);
        client.top_height
    };

    println!();

    tokio::join!(
        client.cryptonight_v0(),
        client.cryptonight_v1(),
        client.cryptonight_v2(),
        client.cryptonight_r(),
        client.randomx(),
    );

    loop {
        let count = TESTED_BLOCK_COUNT.load(Ordering::Acquire);

        if top_height == count {
            println!("finished all PoW, took {}s", now.elapsed().as_secs());
            std::process::exit(0);
        }

        std::thread::sleep(Duration::from_secs(1));
    }
}
