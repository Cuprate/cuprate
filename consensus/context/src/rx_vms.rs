//! RandomX VM Cache
//!
//! This module keeps track of the RandomX VM to calculate the next blocks proof-of-work, if the block needs a randomX VM and potentially
//! more VMs around this height.
//!
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use futures::{stream::FuturesOrdered, StreamExt};
use randomx_rs::{RandomXCache, RandomXError, RandomXFlag, RandomXVM as VmInner};
use rayon::prelude::*;
use thread_local::ThreadLocal;
use tower::ServiceExt;
use tracing::instrument;

use cuprate_consensus_rules::blocks::randomx_seed_height;
use cuprate_consensus_rules::{
    blocks::{is_randomx_seed_height, RandomX, RX_SEEDHASH_EPOCH_BLOCKS},
    HardFork,
};
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain,
};

use crate::{ContextCacheError, Database};

/// The amount of randomX VMs to keep in the cache.
pub const RX_SEEDS_CACHED: usize = 2;

/// A multithreaded randomX VM.
#[derive(Debug)]
pub struct RandomXVm {
    /// These RandomX VMs all share the same cache.
    vms: ThreadLocal<VmInner>,
    /// The RandomX cache.
    cache: RandomXCache,
    /// The flags used to start the RandomX VMs.
    flags: RandomXFlag,
}

impl RandomXVm {
    /// Create a new multithreaded randomX VM with the provided seed.
    pub fn new(seed: &[u8; 32]) -> Result<Self, RandomXError> {
        // TODO: allow passing in flags.
        let flags = RandomXFlag::get_recommended_flags();

        let cache = RandomXCache::new(flags, seed.as_slice())?;

        Ok(Self {
            vms: ThreadLocal::new(),
            cache,
            flags,
        })
    }
}

impl RandomX for RandomXVm {
    type Error = RandomXError;

    fn calculate_hash(&self, buf: &[u8]) -> Result<[u8; 32], Self::Error> {
        self.vms
            .get_or_try(|| VmInner::new(self.flags, Some(self.cache.clone()), None))?
            .calculate_hash(buf)
            .map(|out| out.try_into().unwrap())
    }
}

/// The randomX VMs cache, keeps the VM needed to calculate the current block's proof-of-work hash (if a VM is needed) and a
/// couple more around this VM.
#[derive(Clone, Debug)]
pub struct RandomXVmCache {
    /// The top [`RX_SEEDS_CACHED`] RX seeds.  
    pub seeds: VecDeque<(usize, [u8; 32])>,
    /// The VMs for `seeds` (if after hf 12, otherwise this will be empty).
    pub vms: HashMap<usize, Arc<RandomXVm>>,

    /// A single cached VM that was given to us from a part of Cuprate.
    pub cached_vm: Option<([u8; 32], Arc<RandomXVm>)>,
}

impl RandomXVmCache {
    #[instrument(name = "init_rx_vm_cache", level = "info", skip(database))]
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: usize,
        hf: &HardFork,
        database: D,
    ) -> Result<Self, ContextCacheError> {
        let seed_heights = get_last_rx_seed_heights(chain_height - 1, RX_SEEDS_CACHED);
        let seed_hashes = get_block_hashes(seed_heights.clone(), database).await?;

        tracing::debug!("last {RX_SEEDS_CACHED} randomX seed heights: {seed_heights:?}",);

        let seeds: VecDeque<(usize, [u8; 32])> =
            seed_heights.into_iter().zip(seed_hashes).collect();

        let vms = if hf >= &HardFork::V12 {
            tracing::debug!("Creating RandomX VMs");
            let seeds_clone = seeds.clone();
            rayon_spawn_async(move || {
                seeds_clone
                    .par_iter()
                    .map(|(height, seed)| {
                        (
                            *height,
                            Arc::new(RandomXVm::new(seed).expect("Failed to create RandomX VM!")),
                        )
                    })
                    .collect()
            })
            .await
        } else {
            tracing::debug!("We are before hard-fork 12 randomX VMs are not needed.");
            HashMap::new()
        };

        Ok(Self {
            seeds,
            vms,
            cached_vm: None,
        })
    }

    /// Add a randomX VM to the cache, with the seed it was created with.
    pub fn add_vm(&mut self, vm: ([u8; 32], Arc<RandomXVm>)) {
        self.cached_vm.replace(vm);
    }

    /// Creates a RX VM for an alt chain, looking at the main chain RX VMs to see if we can use one
    /// of them first.
    pub async fn get_alt_vm<D: Database>(
        &self,
        height: usize,
        chain: Chain,
        database: D,
    ) -> Result<Arc<RandomXVm>, ContextCacheError> {
        let seed_height = randomx_seed_height(height);

        let BlockchainResponse::BlockHash(seed_hash) = database
            .oneshot(BlockchainReadRequest::BlockHash(seed_height, chain))
            .await?
        else {
            panic!("Database returned wrong response!");
        };

        for (vm_main_chain_height, vm_seed_hash) in &self.seeds {
            if vm_seed_hash == &seed_hash {
                let Some(vm) = self.vms.get(vm_main_chain_height) else {
                    break;
                };

                return Ok(Arc::clone(vm));
            }
        }

        let alt_vm = rayon_spawn_async(move || Arc::new(RandomXVm::new(&seed_hash).unwrap())).await;

        Ok(alt_vm)
    }

    /// Get the main-chain RandomX VMs.
    pub async fn get_vms(&mut self) -> HashMap<usize, Arc<RandomXVm>> {
        match self.seeds.len().checked_sub(self.vms.len()) {
            // No difference in the amount of seeds to VMs.
            Some(0) => (),
            // One more seed than VM.
            Some(1) => {
                let (seed_height, next_seed_hash) = *self.seeds.front().unwrap();

                let new_vm = 'new_vm_block: {
                    tracing::debug!(
                        "Initializing RandomX VM for seed: {}",
                        hex::encode(next_seed_hash)
                    );

                    // Check if we have been given the RX VM from another part of Cuprate.
                    if let Some((cached_hash, cached_vm)) = self.cached_vm.take() {
                        if cached_hash == next_seed_hash {
                            tracing::debug!("VM was already created.");
                            break 'new_vm_block cached_vm;
                        }
                    };

                    rayon_spawn_async(move || Arc::new(RandomXVm::new(&next_seed_hash).unwrap()))
                        .await
                };

                self.vms.insert(seed_height, new_vm);
            }
            // More than one more seed than VM.
            _ => {
                // this will only happen when syncing and rx activates.
                tracing::debug!("RandomX has activated, initialising VMs");

                let seeds_clone = self.seeds.clone();
                self.vms = rayon_spawn_async(move || {
                    seeds_clone
                        .par_iter()
                        .map(|(height, seed)| {
                            let vm = RandomXVm::new(seed).expect("Failed to create RandomX VM!");
                            let vm = Arc::new(vm);
                            (*height, vm)
                        })
                        .collect()
                })
                .await;
            }
        }

        self.vms.clone()
    }

    /// Removes all the RandomX VMs above the `new_height`.
    pub fn pop_blocks_main_chain(&mut self, new_height: usize) {
        self.seeds.retain(|(height, _)| *height < new_height);
        self.vms.retain(|height, _| *height < new_height);
    }

    /// Add a new block to the VM cache.
    ///
    /// hash is the block hash not the blocks proof-of-work hash.
    pub fn new_block(&mut self, height: usize, hash: &[u8; 32]) {
        if is_randomx_seed_height(height) {
            tracing::debug!("Block {height} is a randomX seed height, adding it to the cache.",);

            self.seeds.push_front((height, *hash));

            if self.seeds.len() > RX_SEEDS_CACHED {
                self.seeds.pop_back();
                // HACK: This is really inefficient but the amount of VMs cached is not a lot.
                self.vms.retain(|height, _| {
                    self.seeds
                        .iter()
                        .any(|(cached_height, _)| height == cached_height)
                });
            }
        }
    }
}

/// Get the last `amount` of RX seeds, the top height returned here will not necessarily be the RX VM for the top block
/// in the chain as VMs include some lag before a seed activates.
pub fn get_last_rx_seed_heights(mut last_height: usize, mut amount: usize) -> Vec<usize> {
    let mut seeds = Vec::with_capacity(amount);
    if is_randomx_seed_height(last_height) {
        seeds.push(last_height);
        amount -= 1;
    }

    for _ in 0..amount {
        if last_height == 0 {
            return seeds;
        }

        // We don't include the lag as we only want seeds not the specific seed for this height.
        let seed_height = (last_height - 1) & !(RX_SEEDHASH_EPOCH_BLOCKS - 1);
        seeds.push(seed_height);
        last_height = seed_height;
    }

    seeds
}

/// Gets the block hashes for the heights specified.
async fn get_block_hashes<D: Database + Clone>(
    heights: Vec<usize>,
    database: D,
) -> Result<Vec<[u8; 32]>, ContextCacheError> {
    let mut fut = FuturesOrdered::new();

    for height in heights {
        let db = database.clone();
        fut.push_back(async move {
            let BlockchainResponse::BlockHash(hash) = db
                .clone()
                .oneshot(BlockchainReadRequest::BlockHash(height, Chain::Main))
                .await?
            else {
                panic!("Database sent incorrect response!");
            };
            Result::<_, ContextCacheError>::Ok(hash)
        });
    }

    let mut res = Vec::new();
    while let Some(hash) = fut.next().await {
        res.push(hash?);
    }
    Ok(res)
}
