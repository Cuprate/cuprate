use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

mod rpc;

pub static TESTED_BLOCK_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static TESTED_TX_COUNT: AtomicUsize = AtomicUsize::new(0);

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

    let ranges = (0..top_height)
        .collect::<Vec<usize>>()
        .chunks(100_000)
        .map(<[usize]>::to_vec)
        .collect::<Vec<Vec<usize>>>();

    println!("ranges: [");
    for range in &ranges {
        println!(
            "    ({}..{}),",
            range.first().unwrap(),
            range.last().unwrap()
        );
    }

    println!("]\n");

    let iter = ranges.into_iter().map(move |range| {
        let c = client.clone();
        async move {
            tokio::task::spawn_blocking(move || async move {
                c.get_block_test_batch(range.into_iter().collect()).await;
            })
            .await
            .unwrap()
            .await;
        }
    });

    futures::future::join_all(iter).await;

    loop {
        let block_count = TESTED_BLOCK_COUNT.load(Ordering::Acquire);
        let tx_count = TESTED_TX_COUNT.load(Ordering::Acquire);

        if top_height == block_count {
            println!(
                "finished processing: blocks: {block_count}/{top_height}, txs: {tx_count}, took {}s",
                now.elapsed().as_secs()
            );
            std::process::exit(0);
        }

        std::thread::sleep(Duration::from_secs(1));
    }
}
