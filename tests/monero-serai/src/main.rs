use std::{
    io::Write,
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
        url
    } else {
        "http://127.0.0.1:18081".to_string()
    };
    println!("rpc_url: {rpc_url}");

    let client = rpc::RpcClient::new(rpc_url).await;
    let top_height = client.top_height;

    let ranges = (0..top_height)
        .collect::<Vec<usize>>()
        .chunks(100_000)
        .map(<[usize]>::to_vec)
        .collect::<Vec<Vec<usize>>>();

    println!("ranges: ");
    for range in &ranges {
        println!("[{}..{}]", range.first().unwrap(), range.last().unwrap());
    }

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
