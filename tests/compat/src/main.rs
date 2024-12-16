#![allow(unreachable_pub, reason = "This is a binary, everything `pub` is ok")]

mod cli;
mod constants;
mod cryptonight;
mod randomx;
mod rpc;
mod types;
mod verify;

use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() {
    let now = Instant::now();

    // Parse CLI args.
    let cli::Args {
        rpc_url,
        update,
        threads,
    } = cli::Args::get();

    // Set-up RPC client.
    let client = rpc::RpcClient::new(rpc_url).await;
    let top_height = client.top_height;
    println!("top_height: {top_height}");
    println!();

    // Test.
    let (tx, rx) = crossbeam::channel::unbounded();
    verify::spawn_verify_pool(threads, update, top_height, rx);
    client.test(top_height, tx).await;

    // Wait for other threads to finish.
    loop {
        let count = constants::TESTED_BLOCK_COUNT.load(Ordering::Acquire);

        if top_height == count {
            println!("Finished, took {}s", now.elapsed().as_secs());
            std::process::exit(0);
        }

        std::thread::sleep(Duration::from_secs(1));
    }
}
