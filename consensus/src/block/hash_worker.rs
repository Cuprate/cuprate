use cryptonight_cuprate::{
    cryptonight_hash_r, cryptonight_hash_v0, cryptonight_hash_v1, cryptonight_hash_v2,
};

use crate::{ConsensusError, HardFork};

/// Calcualtes the POW hash of this block.
pub fn calculate_pow_hash(
    buf: &[u8],
    height: u64,
    hf: &HardFork,
) -> Result<[u8; 32], ConsensusError> {
    if height == 202612 {
        return Ok(
            hex::decode("84f64766475d51837ac9efbef1926486e58563c95a19fef4aec3254f03000000")
                .unwrap()
                .try_into()
                .unwrap(),
        );
    }

    Ok(if hf.in_range(&HardFork::V1, &HardFork::V7) {
        cryptonight_hash_v0(buf)
    } else if hf == &HardFork::V7 {
        cryptonight_hash_v1(buf).map_err(|_| ConsensusError::BlockPOWInvalid)?
    } else if hf.in_range(&HardFork::V8, &HardFork::V10) {
        cryptonight_hash_v2(buf)
    } else if hf.in_range(&HardFork::V10, &HardFork::V12) {
        cryptonight_hash_r(buf, height)
    } else {
        todo!("RandomX")
    })
}
