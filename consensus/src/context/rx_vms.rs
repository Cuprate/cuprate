use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use futures::{stream::FuturesOrdered, StreamExt};
use randomx_rs::{RandomXCache, RandomXError, RandomXFlag, RandomXVM as VMInner};
use rayon::prelude::*;
use thread_local::ThreadLocal;
use tower::ServiceExt;

use cuprate_helper::asynch::rayon_spawn_async;
use monero_consensus::{
    blocks::{is_randomx_seed_height, RandomX, RX_SEEDHASH_EPOCH_BLOCKS},
    HardFork,
};

use crate::{Database, DatabaseRequest, DatabaseResponse, ExtendedConsensusError};

const RX_SEEDS_CACHED: usize = 2;

#[derive(Debug)]
pub struct RandomXVM {
    vms: ThreadLocal<VMInner>,
    cache: RandomXCache,
    flags: RandomXFlag,
}

impl RandomXVM {
    pub fn new(seed: &[u8; 32]) -> Result<Self, RandomXError> {
        let flags = RandomXFlag::get_recommended_flags();

        let cache = RandomXCache::new(flags, seed.as_slice())?;

        Ok(RandomXVM {
            vms: ThreadLocal::new(),
            cache,
            flags,
        })
    }
}

impl RandomX for RandomXVM {
    type Error = RandomXError;

    fn calculate_hash(&self, buf: &[u8]) -> Result<[u8; 32], Self::Error> {
        self.vms
            .get_or_try(|| VMInner::new(self.flags, Some(self.cache.clone()), None))?
            .calculate_hash(buf)
            .map(|out| out.try_into().unwrap())
    }
}

#[derive(Clone, Debug)]
pub struct RandomXVMCache {
    pub(crate) seeds: VecDeque<(u64, [u8; 32])>,
    pub(crate) vms: HashMap<u64, Arc<RandomXVM>>,

    pub(crate) cached_vm: Option<([u8; 32], Arc<RandomXVM>)>,
}

impl RandomXVMCache {
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: u64,
        hf: &HardFork,
        database: D,
    ) -> Result<Self, ExtendedConsensusError> {
        let seed_heights = get_last_rx_seed_heights(chain_height - 1, RX_SEEDS_CACHED);
        let seed_hashes = get_block_hashes(seed_heights.clone(), database).await?;

        let seeds: VecDeque<(u64, [u8; 32])> = seed_heights.into_iter().zip(seed_hashes).collect();

        let vms = if hf >= &HardFork::V12 {
            let seeds_clone = seeds.clone();
            rayon_spawn_async(move || {
                seeds_clone
                    .par_iter()
                    .map(|(height, seed)| {
                        (
                            *height,
                            Arc::new(RandomXVM::new(seed).expect("Failed to create RandomX VM!")),
                        )
                    })
                    .collect()
            })
            .await
        } else {
            HashMap::new()
        };

        Ok(RandomXVMCache {
            seeds,
            vms,
            cached_vm: None,
        })
    }

    pub fn add_vm(&mut self, vm: ([u8; 32], Arc<RandomXVM>)) {
        self.cached_vm.replace(vm);
    }

    pub fn get_vms(&self) -> HashMap<u64, Arc<RandomXVM>> {
        self.vms.clone()
    }

    pub async fn new_block(&mut self, height: u64, hash: &[u8; 32], hf: &HardFork) {
        let should_make_vms = hf >= &HardFork::V12;
        if should_make_vms && self.vms.len() != self.seeds.len() {
            // this will only happen when syncing and rx activates.
            let seeds_clone = self.seeds.clone();
            self.vms = rayon_spawn_async(move || {
                seeds_clone
                    .par_iter()
                    .map(|(height, seed)| {
                        (
                            *height,
                            Arc::new(RandomXVM::new(seed).expect("Failed to create RandomX VM!")),
                        )
                    })
                    .collect()
            })
            .await
        }

        if is_randomx_seed_height(height) {
            self.seeds.push_front((height, *hash));

            if should_make_vms {
                let new_vm = 'new_vm_block: {
                    if let Some((cached_hash, cached_vm)) = self.cached_vm.take() {
                        if &cached_hash == hash {
                            break 'new_vm_block cached_vm;
                        }
                    };

                    let hash_clone = *hash;
                    rayon_spawn_async(move || Arc::new(RandomXVM::new(&hash_clone).unwrap())).await
                };

                self.vms.insert(height, new_vm);
            }

            if self.seeds.len() > RX_SEEDS_CACHED {
                self.seeds.pop_back();
                // TODO: This is really not efficient but the amount of VMs cached is not a lot.
                self.vms.retain(|height, _| {
                    self.seeds
                        .iter()
                        .any(|(cached_height, _)| height == cached_height)
                })
            }
        }
    }
}

pub(crate) fn get_last_rx_seed_heights(mut last_height: u64, mut amount: usize) -> Vec<u64> {
    let mut seeds = Vec::with_capacity(amount);
    if is_randomx_seed_height(last_height) {
        seeds.push(last_height);
        amount -= 1;
    }

    for _ in 0..amount {
        if last_height == 0 {
            return seeds;
        }

        // We don't include the lag as we only want seeds not the specific seed fo this height.
        let seed_height = (last_height - 1) & !(RX_SEEDHASH_EPOCH_BLOCKS - 1);
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
