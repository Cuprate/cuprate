use hex_literal::hex;
use tower::{Service, ServiceExt};

use cuprate_types::blockchain::{BCReadRequest, BCResponse};

use cuprate_blockchain::{
    ConcreteEnv,
    config::ConfigBuilder,
    Env,
    service::DatabaseReadHandle,
};

use cuprate_fast_sync::{hash_of_hashes, BlockId, HashOfHashes};
use std::{
    fmt::Write,
    fs::write,
};

const BATCH_SIZE: u64 = 512;

async fn read_batch(handle: &mut DatabaseReadHandle, height_from: u64) -> Vec<BlockId> {
    let mut block_ids = Vec::<BlockId>::with_capacity(BATCH_SIZE as usize);

    for height in height_from..(height_from + BATCH_SIZE)  {
        let request = BCReadRequest::BlockHash(height);
        let response_channel = handle.ready().await.unwrap().call(request);
        let response = response_channel.await.unwrap();

        match response {
            BCResponse::BlockHash(block_id) => block_ids.push(block_id),
            _ => unreachable!(),
        }
    }

    block_ids
}

fn generate_hex(hashes: &[HashOfHashes]) -> String {
    let mut s = String::new();

    writeln!(&mut s, "[").unwrap();

    for hash in hashes {
        write!(&mut s, "\thex!(\"").unwrap();
        for byte in hash {
            write!(&mut s, "{:02x}", byte).unwrap();
        }
        writeln!(&mut s, "\"),").unwrap();
    }

    writeln!(&mut s, "]").unwrap();

    s
} 

#[tokio::main]
async fn main() { 
    let config = ConfigBuilder::new().build();

    let (mut read_handle, _) = cuprate_blockchain::service::init(config).unwrap();

    let mut hashes_of_hashes = Vec::new();

    let height_target = 5120u64; // TODO make a CLI option

    let mut height = 0u64;

    while height < height_target {
        let block_ids = read_batch(&mut read_handle, height).await;
        let hash = hash_of_hashes(block_ids.as_slice());
        hashes_of_hashes.push(hash);
        height += BATCH_SIZE;
    }

    drop(read_handle);
    
    let generated = generate_hex(&hashes_of_hashes);
    write("src/data/hashes_of_hashes", generated).unwrap();
}
