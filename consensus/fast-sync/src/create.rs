#![expect(
    unused_crate_dependencies,
    reason = "binary shares same Cargo.toml as library"
)]

use clap::Parser;
use std::fs::write;
use std::sync::Arc;
use tower::{Service, ServiceExt};

use cuprate_blockchain::config::Config;
use cuprate_blockchain::{service::BlockchainReadHandle, DbResult};
use cuprate_hex::Hex;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain,
};

use cuprate_fast_sync::FAST_SYNC_BATCH_LEN;
use cuprate_helper::fs::CUPRATE_DATA_DIR;

async fn read_batch(
    handle: &mut BlockchainReadHandle,
    height_from: usize,
) -> DbResult<Vec<[u8; 32]>> {
    let request = BlockchainReadRequest::BlockHashInRange(
        height_from..(height_from + FAST_SYNC_BATCH_LEN),
        Chain::Main,
    );
    let response_channel = handle.ready().await?.call(request);
    let response = response_channel.await?;

    let BlockchainResponse::BlockHashInRange(block_ids) = response else {
        unreachable!()
    };

    Ok(block_ids)
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    height: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let height_target = args.height;

    let config = Config::default();
    let fjall_dir = CUPRATE_DATA_DIR.to_path_buf().join("fjall");

    let fjall = fjall::Database::builder(fjall_dir).open().unwrap();

    let thread_pool = Arc::new(rayon::ThreadPoolBuilder::new().build().unwrap());

    let (mut read_handle, _, _) =
        cuprate_blockchain::service::init_with_pool(&config, fjall, thread_pool).unwrap();

    let mut hashes_of_hashes = Vec::new();

    let mut height = 0_usize;

    while (height + FAST_SYNC_BATCH_LEN) < height_target {
        if let Ok(block_ids) = read_batch(&mut read_handle, height).await {
            let hash = hash_of_hashes(block_ids.as_slice());
            hashes_of_hashes.push(Hex(hash));
        } else {
            println!("Failed to read next batch from database");
            break;
        }
        height += FAST_SYNC_BATCH_LEN;

        println!("height: {height}");
    }

    drop(read_handle);

    write(
        "fast_sync_hashes.json",
        serde_json::to_string_pretty(&hashes_of_hashes).unwrap(),
    )
    .unwrap();

    println!("Generated hashes up to block height {height}");
}

pub fn hash_of_hashes(hashes: &[[u8; 32]]) -> [u8; 32] {
    blake3::hash(hashes.concat().as_slice()).into()
}
