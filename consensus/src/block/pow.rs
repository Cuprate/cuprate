use crypto_bigint::{CheckedMul, U256};

pub mod difficulty;

#[derive(Debug)]
pub struct BlockPOWInfo {
    pub timestamp: u64,
    pub cumulative_difficulty: u128,
}

pub fn check_block_pow(hash: &[u8; 32], difficulty: u128) -> bool {
    let int_hash = U256::from_le_slice(hash);

    let difficulty = U256::from_u128(difficulty);

    int_hash.checked_mul(&difficulty).is_some().unwrap_u8() == 1
}
