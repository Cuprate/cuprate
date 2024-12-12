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
        url
    } else {
        "http://127.0.0.1:18081".to_string()
    };
    println!("rpc_url: {rpc_url}");

    let client = rpc::RpcClient::new(rpc_url).await;
    let top_height = client.top_height;

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
