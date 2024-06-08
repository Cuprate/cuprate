use sha3::{Digest, Keccak256};

pub type BlockId = [u8; 32];
pub type HashOfHashes = [u8; 32];

pub fn hash_of_hashes(hashes: &[BlockId]) -> HashOfHashes {
    Keccak256::digest(hashes.concat().as_slice()).into()
}
