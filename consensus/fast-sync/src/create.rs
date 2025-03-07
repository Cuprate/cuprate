#![expect(
    unused_crate_dependencies,
    reason = "binary shares same Cargo.toml as library"
)]

use std::fs::write;

use clap::Parser;
use tower::{Service, ServiceExt};

use cuprate_blockchain::{
    config::ConfigBuilder, cuprate_database::DbResult, service::BlockchainReadHandle,
};
use cuprate_types::{
    Chain,
    blockchain::{BlockchainReadRequest, BlockchainResponse},
};

const BATCH_SIZE: usize = 512;

async fn read_batch(
    handle: &mut BlockchainReadHandle,
    height_from: usize,
) -> DbResult<Vec<[u8; 32]>> {
    let request = BlockchainReadRequest::BlockHashInRange(
        height_from..(height_from + BATCH_SIZE),
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

    let config = ConfigBuilder::new().build();

    let (mut read_handle, _, _) = cuprate_blockchain::service::init(config).unwrap();

    let mut hashes_of_hashes = Vec::new();

    let mut height = 0_usize;

    while (height + BATCH_SIZE) < height_target {
        if let Ok(block_ids) = read_batch(&mut read_handle, height).await {
            let hash = hash_of_hashes(block_ids.as_slice());
            hashes_of_hashes.push(hash);
        } else {
            println!("Failed to read next batch from database");
            break;
        }
        height += BATCH_SIZE;

        println!("height: {height}");
    }

    drop(read_handle);

    write(
        "data/fast_sync_hashes.bin",
        hashes_of_hashes.concat().as_slice(),
    )
    .expect("Could not write file");

    println!("Generated hashes up to block height {height}");
}

pub fn hash_of_hashes(hashes: &[[u8; 32]]) -> [u8; 32] {
    blake3::hash(hashes.concat().as_slice()).into()
}
