use std::collections::VecDeque;

use futures::{stream::FuturesOrdered, StreamExt};
use tower::ServiceExt;

use monero_consensus::blocks::{is_randomx_seed_height, randomx_seed_height};

use crate::{Database, DatabaseRequest, DatabaseResponse, ExtendedConsensusError};

const RX_SEEDS_CACHED: usize = 3;

#[derive(Clone, Debug)]
pub struct RandomXSeed {
    seeds: VecDeque<(u64, [u8; 32])>,
}

impl RandomXSeed {
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: u64,
        database: D,
    ) -> Result<Self, ExtendedConsensusError> {
        let seed_heights = get_last_rx_seed_heights(chain_height - 1, RX_SEEDS_CACHED);
        let seed_hashes = get_block_hashes(seed_heights.clone(), database).await?;

        Ok(RandomXSeed {
            seeds: seed_heights.into_iter().zip(seed_hashes).collect(),
        })
    }

    pub fn get_seeds_hash(&self, seed_height: u64) -> [u8; 32] {
        for (height, seed) in self.seeds.iter() {
            if seed_height == *height {
                return *seed;
            }
        }

        tracing::error!(
            "Current seeds: {:?}, asked for: {}",
            self.seeds,
            seed_height
        );
        panic!("RX seed cache was not updated or was asked for a block too old.")
    }

    pub fn get_rx_seed(&self, height: u64) -> [u8; 32] {
        let seed_height = randomx_seed_height(height);
        tracing::warn!(
            "Current seeds: {:?}, asked for: {}",
            self.seeds,
            seed_height
        );

        self.get_seeds_hash(seed_height)
    }

    pub fn new_block(&mut self, height: u64, hash: &[u8; 32]) {
        if is_randomx_seed_height(height) {
            for (got_height, _) in self.seeds.iter() {
                if *got_height == height {
                    return;
                }
            }

            self.seeds.push_front((height, *hash));

            if self.seeds.len() > RX_SEEDS_CACHED {
                self.seeds.pop_back();
            }
        }
    }
}

fn get_last_rx_seed_heights(mut last_height: u64, mut amount: usize) -> Vec<u64> {
    let mut seeds = Vec::with_capacity(amount);
    if is_randomx_seed_height(last_height) {
        seeds.push(last_height);
        amount -= 1;
    }

    for _ in 0..amount {
        if last_height == 0 {
            return seeds;
        }

        let seed_height = randomx_seed_height(last_height);
        seeds.push(seed_height);
        last_height = seed_height
    }

    seeds
}

async fn get_block_hashes<D: Database + Clone>(
    heights: Vec<u64>,
    database: D,
) -> Result<Vec<[u8; 32]>, ExtendedConsensusError> {
    let mut fut = FuturesOrdered::new();

    for height in heights {
        let db = database.clone();
        fut.push_back(async move {
            let DatabaseResponse::BlockHash(hash) = db
                .clone()
                .oneshot(DatabaseRequest::BlockHash(height))
                .await?
            else {
                panic!("Database sent incorrect response!");
            };
            Result::<_, ExtendedConsensusError>::Ok(hash)
        });
    }

    let mut res = Vec::new();
    while let Some(hash) = fut.next().await {
        res.push(hash?);
    }
    Ok(res)
}
