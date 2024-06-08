use sha3::{Digest, Keccak256};

pub(crate) type BlockId = [u8; 32];
pub(crate) type HashOfHashes = [u8; 32];

pub(crate) fn hash_of_hashes(hashes: &[BlockId]) -> HashOfHashes {
    Keccak256::digest(hashes.concat().as_slice()).into()
}
