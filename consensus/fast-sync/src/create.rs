#![expect(
    unused_crate_dependencies,
    reason = "binary shares same Cargo.toml as library"
)]

use std::{fs::write, path::PathBuf};

use clap::Parser;
use tower::{Service, ServiceExt};

use cuprate_blockchain::{
    config::ConfigBuilder, cuprate_database::DbResult, service::BlockchainReadHandle,
};
use cuprate_hex::Hex;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain,
};

use cuprate_fast_sync::FAST_SYNC_BATCH_LEN;

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
    /// Hashes will be created up to this height
    /// (in batches of [`FAST_SYNC_BATCH_LEN`]).
    #[arg(long)]
    height: usize,

    /// Optional path towards the blockchain database directory.
    ///
    /// If not provided, the default directory will be used.
    #[arg(long)]
    blockchain_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let height_target = args.height;

    let config = {
        let c = ConfigBuilder::new();
        if let Some(dir) = args.blockchain_dir {
            c.data_directory(dir)
        } else {
            c
        }
        .build()
    };

    println!("Height: {height_target}");
    println!(
        "Blockchain directory: {}",
        config.db_config.db_directory().display()
    );

    let (mut read_handle, _, _) = cuprate_blockchain::service::init(config).unwrap();

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
