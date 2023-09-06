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

#[test]
fn chekc() {
    let hash = hex::decode("5aeebb3de73859d92f3f82fdb97286d81264ecb72a42e4b9f1e6d62eb682d7c0")
        .unwrap()
        .try_into()
        .unwrap();
    let diff = 257344482654;

    assert!(check_block_pow(&hash, diff))
}
