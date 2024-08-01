use std::{fmt::Write, fs::write};

use clap::Parser;
use tower::{Service, ServiceExt};

use cuprate_blockchain::{
    config::ConfigBuilder, cuprate_database::RuntimeError, service::DatabaseReadHandle,
};
use cuprate_types::{
    blockchain::{BCReadRequest, BCResponse},
    Chain,
};

use cuprate_fast_sync::{hash_of_hashes, BlockId, HashOfHashes};

const BATCH_SIZE: usize = 512;

async fn read_batch(
    handle: &mut DatabaseReadHandle,
    height_from: usize,
) -> Result<Vec<BlockId>, RuntimeError> {
    let mut block_ids = Vec::<BlockId>::with_capacity(BATCH_SIZE);

    for height in height_from..(height_from + BATCH_SIZE) {
        let request = BCReadRequest::BlockHash(height, Chain::Main);
        let response_channel = handle.ready().await?.call(request);
        let response = response_channel.await?;

        match response {
            BCResponse::BlockHash(block_id) => block_ids.push(block_id),
            _ => unreachable!(),
        }
    }

    Ok(block_ids)
}

fn generate_hex(hashes: &[HashOfHashes]) -> String {
    let mut s = String::new();

    writeln!(&mut s, "[").unwrap();

    for hash in hashes {
        writeln!(&mut s, "\thex!(\"{}\"),", hex::encode(hash)).unwrap();
    }

    writeln!(&mut s, "]").unwrap();

    s
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    height: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let height_target = args.height;

    let config = ConfigBuilder::new().build();

    let (mut read_handle, _) = cuprate_blockchain::service::init(config).unwrap();

    let mut hashes_of_hashes = Vec::new();

    let mut height = 0_usize;

    while height < height_target {
        match read_batch(&mut read_handle, height).await {
            Ok(block_ids) => {
                let hash = hash_of_hashes(block_ids.as_slice());
                hashes_of_hashes.push(hash);
            }
            Err(_) => {
                println!("Failed to read next batch from database");
                break;
            }
        }
        height += BATCH_SIZE;
    }

    drop(read_handle);

    let generated = generate_hex(&hashes_of_hashes);
    write("src/data/hashes_of_hashes", generated).expect("Could not write file");

    println!("Generated hashes up to block height {}", height);
}
