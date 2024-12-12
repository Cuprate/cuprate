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

    let top_height = rpc::RpcClient::top_height(rpc_node_url.clone()).await;
    println!("top_height: {top_height}");
    assert!(top_height > 3301441, "node is behind");

    let ranges = (0..top_height)
        .collect::<Vec<usize>>()
        .chunks(100_000)
        .map(<[usize]>::to_vec)
        .collect::<Vec<Vec<usize>>>();

    println!("ranges: ");
    for range in &ranges {
        println!("[{}..{}]", range.first().unwrap(), range.last().unwrap());
    }

    let rpc_client = rpc::RpcClient::new(rpc_node_url);

    let iter = ranges.into_iter().map(move |range| {
        let c = rpc_client.clone();
        async move {
            tokio::task::spawn_blocking(move || async move {
                c.get_block_test_batch(range.into_iter().collect()).await;
            })
            .await
            .unwrap()
            .await;
        }
    });

    std::thread::spawn(move || {
        let mut count = 0;

        #[expect(clippy::cast_precision_loss)]
        while count != top_height {
            let c = TESTED_BLOCK_COUNT.load(Ordering::Acquire);
            count = c;

            println!(
                "blocks processed ... {c}/{top_height} ({:.2}%)",
                (c as f64 / top_height as f64) * 100.0
            );

            std::thread::sleep(Duration::from_millis(250));
        }

        println!("finished all blocks, took: {}s", now.elapsed().as_secs());
        std::process::exit(0);
    });

    futures::future::join_all(iter).await;
    std::thread::park();
}
