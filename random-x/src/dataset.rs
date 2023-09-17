use std::sync::{Arc, RwLock};

use argon2::{Algorithm, Argon2, Block, Params, Version};
#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::blake2_generator::Blake2Generator;
use crate::{
    config::{
        RANDOMX_ARGON_ITERATIONS, RANDOMX_ARGON_LANES, RANDOMX_ARGON_MEMORY, RANDOMX_ARGON_SALT,
        RANDOMX_CACHE_ACCESSES, RANDOMX_DATASET_SIZE,
    },
    registers::{RGroupRegisterID, RGroupRegisters},
    superscalar::SSProgram,
};

/// Generates the memory blocks used in the cache
fn argon2_blocks(key: &[u8]) -> Box<[Block]> {
    let params = Params::new(
        RANDOMX_ARGON_MEMORY,
        RANDOMX_ARGON_ITERATIONS,
        RANDOMX_ARGON_LANES,
        None,
    )
    .unwrap();

    let numb_blocks: usize = (RANDOMX_ARGON_LANES * RANDOMX_ARGON_MEMORY)
        .try_into()
        .unwrap();

    let mut blocks = vec![Block::new(); numb_blocks].into_boxed_slice();

    let argon = Argon2::new(Algorithm::Argon2d, Version::V0x13, params);

    argon
        .fill_memory(key, RANDOMX_ARGON_SALT, &mut blocks)
        .unwrap();
    blocks
}

/// The Cache.
///
/// The cache is used during light verification.
/// Internally this struct is a wrapper around an [`Arc`] internal cache, this allows
/// cheep clones and allows the cache to be shared between VMs on different threads.
#[derive(Debug, Clone)]
pub struct Cache {
    internal_cache: Arc<RwLock<InternalCache>>,
}

impl Cache {
    /// Initialises the cache with the provided key.
    ///
    /// The key must be between 1-60 bytes (inclusive) otherwise this will panic.
    pub fn init(key: &[u8]) -> Self {
        let internal_cache = InternalCache::init(key);
        Cache {
            internal_cache: Arc::new(RwLock::new(internal_cache)),
        }
    }
}

/// The internal cache structure, used during light verification.
#[derive(Debug)]

struct InternalCache {
    memory_blocks: Box<[Block]>,
    programs: Vec<SSProgram>,
}

impl InternalCache {
    fn init(key: &[u8]) -> Self {
        let memory_blocks = argon2_blocks(key);

        let mut blake_gen = Blake2Generator::new(key, 0);

        let programs = (0..RANDOMX_CACHE_ACCESSES)
            .map(|_| SSProgram::generate(&mut blake_gen))
            .collect::<Vec<_>>();

        InternalCache {
            memory_blocks,
            programs,
        }
    }

    /// Gets an item from the cache at the specified index.
    fn get_item(&self, idx: usize) -> [u64; 8] {
        // one item is 8 u64s
        // mask = (blocks in cache * bytes in a block / size of item) minus one.
        let mask = (self.memory_blocks.len() * 1024 / 64) - 1;
        // and the idx with the mask this is the same as doing mod (self.memory_blocks.len() * 1024 / 64)
        let idx = idx & mask;

        // block_idx = idx divided by amount of items in a block
        let block_idx = idx / (1024 / 64);
        // idx * 8 is to get the idx of a single u64
        // we mask with amount of u64s in a block minus 1 which is the same as doing
        // mod the amount of instructions in a block.
        let block_u64_start = (idx * 8) & 127;
        // The plus 8 cannot overflow as (idx * 8) & 127 wont give a number bigger than 120
        return self.memory_blocks[block_idx].as_ref()[block_u64_start..block_u64_start + 8]
            .try_into()
            .unwrap();
    }

    /// Generates the dataset item at the specified index.
    fn init_data_set_item(&self, item_number: usize) -> [u64; 8] {
        let mut registers = RGroupRegisters::default();
        registers.set(
            &RGroupRegisterID::R0,
            (TryInto::<u64>::try_into(item_number).unwrap() + 1_u64)
                .wrapping_mul(6364136223846793005_u64),
        );

        let mut init_reg = |dst, val: u64| {
            registers.apply_to_dst_with_src(&dst, &RGroupRegisterID::R0, |_, src| src ^ val)
        };

        init_reg(RGroupRegisterID::R1, 9298411001130361340);
        init_reg(RGroupRegisterID::R2, 12065312585734608966);
        init_reg(RGroupRegisterID::R3, 9306329213124626780);
        init_reg(RGroupRegisterID::R4, 5281919268842080866);
        init_reg(RGroupRegisterID::R5, 10536153434571861004);
        init_reg(RGroupRegisterID::R6, 3398623926847679864);
        init_reg(RGroupRegisterID::R7, 9549104520008361294);

        let mut cache_index = item_number;

        for program in &self.programs {
            program.execute(&mut registers);

            let cache_item = self.get_item(cache_index);
            for (reg_id, item) in RGroupRegisterID::iter().zip(cache_item) {
                registers.apply_to_dst(&reg_id, |dst| dst ^ item);
            }

            cache_index = registers
                .get(&program.reg_with_max_latency())
                .try_into()
                .expect("u64 does not fit into usize");
        }
        registers.inner()
    }
}

/// The Dataset used during mining.
///
/// Internally this struct is a wrapper around an [`Arc`] internal dataset, this allows
/// cheep clones and allows the dataset to be shared between VMs on different threads.
#[derive(Debug, Clone)]
pub struct Dataset {
    internal_dataset: Arc<RwLock<InternalDataset>>,
}

impl Dataset {
    /// Initialises the dataset with the provided key.
    ///
    /// The key must be between 1-60 bytes (inclusive) otherwise this will panic.
    ///
    /// This is very computationally intense so might take a long time to complete.
    pub fn init(key: &[u8]) -> Dataset {
        let internal_dataset = InternalDataset::init(key);
        Dataset {
            internal_dataset: Arc::new(RwLock::new(internal_dataset)),
        }
    }
}

/// The internal dataset used during mining.
#[derive(Debug)]
struct InternalDataset {
    dataset: Vec<[u64; 8]>,
}

impl InternalDataset {
    fn init(key: &[u8]) -> InternalDataset {
        let cache = InternalCache::init(key);

        #[cfg(feature = "rayon")]
        let dataset: Vec<[u64; 8]> = (0..RANDOMX_DATASET_SIZE / (64 * 8))
            .into_par_iter()
            .map(|i| cache.init_data_set_item(i))
            .collect();

        #[cfg(not(feature = "rayon"))]
        let dataset: Vec<[u64; 8]> = (0..RANDOMX_DATASET_SIZE / (64 * 8))
            .map(|i| cache.init_data_set_item(i))
            .collect();

        Self { dataset }
    }
}
