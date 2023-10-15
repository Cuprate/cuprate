use crypto_bigint::{CheckedMul, U256};
use cryptonight_cuprate::{cryptonight_hash, Variant};

use crate::{hardforks::HardFork, ConsensusError};

#[derive(Debug)]
pub struct BlockPOWInfo {
    pub timestamp: u64,
    pub cumulative_difficulty: u128,
}

/// Returns if the blocks POW hash is valid for the current difficulty.
///
/// See: https://cuprate.github.io/monero-book/consensus_rules/blocks/difficulty.html#checking-a-blocks-proof-of-work
pub fn check_block_pow(hash: &[u8; 32], difficulty: u128) -> bool {
    let int_hash = U256::from_le_slice(hash);

    let difficulty = U256::from_u128(difficulty);

    int_hash.checked_mul(&difficulty).is_some().unwrap_u8() == 1
}

/// Calcualtes the POW hash of this block.
pub fn calculate_pow_hash(buf: &[u8], height: u64, hf: &HardFork) -> [u8; 32] {
    if height == 202612 {
        return hex::decode("84f64766475d51837ac9efbef1926486e58563c95a19fef4aec3254f03000000")
            .unwrap()
            .try_into()
            .unwrap();
    }

    if hf.in_range(&HardFork::V1, &HardFork::V7) {
        cryptonight_hash(buf, &Variant::V0)
        //cryptonight_hash::cryptonight_hash(buf, &Variant::V0)
    } else if hf == &HardFork::V7 {
        cryptonight_hash(buf, &Variant::V1)
    } else if hf.in_range(&HardFork::V8, &HardFork::V10) {
        cryptonight_hash(buf, &Variant::V2)
    } else if hf.in_range(&HardFork::V10, &HardFork::V12) {
        cryptonight_hash(buf, &Variant::R { height })
    } else {
        todo!("RandomX")
    }
}
