use argon2::{Algorithm, Argon2, Block, Params, Version};
use std::sync::{Arc, RwLock};

use crate::blake2_generator::Blake2Generator;
use crate::{
    config::{
        RANDOMX_ARGON_ITERATIONS, RANDOMX_ARGON_LANES, RANDOMX_ARGON_MEMORY, RANDOMX_ARGON_SALT,
        RANDOMX_CACHE_ACCESSES,
    },
    superscalar::SSProgram,
};

trait Dataset {}

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
}

pub struct Cache {
    internal_cache: Arc<RwLock<InternalCache>>,
}

struct InternalDataset {
    dataset: Vec<u64>,
}

impl InternalDataset {
    fn init(key: &[u8]) -> InternalDataset {
        let cache = InternalCache::init(key);
        let

    }
}

fn init_data_set_item(cache: &InternalCache, item: u64) -> [u64; 8] {
    let
}

// 12118971377224777581
#[test]
fn init() {
    let mem = InternalCache::init(&[5]);

    println!("{:?}", mem.memory_blocks[1000])
}
