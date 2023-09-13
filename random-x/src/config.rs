/// Target latency for SuperscalarHash (in cycles of the reference CPU).
pub(crate) const RANDOMX_SUPERSCALAR_LATENCY: usize = 170;

pub(crate) const SUPERSCALAR_MAX_SIZE: usize = 3 * RANDOMX_SUPERSCALAR_LATENCY + 2;

/// Dataset base size in bytes. Must be a power of 2.
pub(crate) const RANDOMX_DATASET_BASE_SIZE: usize = 2147483648;

pub(crate) const RANDOMX_DATASET_EXTRA_SIZE: usize = 33554368;

pub(crate) const RANDOMX_DATASET_SIZE: usize =
    RANDOMX_DATASET_BASE_SIZE + RANDOMX_DATASET_EXTRA_SIZE;

pub(crate) const RANDOMX_ARGON_LANES: u32 = 1;

pub(crate) const RANDOMX_ARGON_ITERATIONS: u32 = 3;

pub(crate) const RANDOMX_ARGON_MEMORY: u32 = 262144;

pub(crate) const RANDOMX_ARGON_SALT: &[u8] = b"RandomX\x03";

pub(crate) const RANDOMX_CACHE_ACCESSES: usize = 8;
