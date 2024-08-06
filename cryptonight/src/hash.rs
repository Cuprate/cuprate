use std::cmp::PartialEq;
use std::io::Write;

use sha3::Digest;

use crate::{cnaes, hash_v4 as v4};

const MEMORY: usize = 1 << 21; // 2MB scratchpad
const AES_BLOCK_SIZE: usize = 16;
const AES_KEY_SIZE: usize = 32;
const INIT_SIZE_BLK: usize = 8;
const INIT_SIZE_BYTE: usize = INIT_SIZE_BLK * AES_BLOCK_SIZE;

#[derive(PartialEq, Eq)]
enum Variant {
    V0,
    V1,
    V2,
    R,
}

struct CnSlowHashState {
    b: [u8; 200],
}

impl Default for CnSlowHashState {
    fn default() -> Self {
        CnSlowHashState {
            b: [0; 200],
        }
    }
}

impl CnSlowHashState {
    fn get_keccak_bytes(&self) -> &[u8; 200] {
        &self.b
    }

    fn get_keccak_bytes_mut(&mut self) -> &mut [u8; 200] {
        &mut self.b
    }

    fn get_keccak_word(&self, index: usize) -> u64 {
        let start = index * 8;
        let end = start + 8;
        u64::from_le_bytes(self.b[start..end].try_into().unwrap())
    }

    fn get_k(&mut self) -> &[u8] {
        &self.b[0..64]
    }

    fn get_init(&mut self) -> &[u8] {
        &self.b[64..64 + INIT_SIZE_BYTE]
    }
}

/// Performs an XOR operation on the first 8-bytes of two slices placing
/// the result in the 'left' slice.
#[inline]
fn xor64(left: &mut [u8], right: &[u8]) {
    debug_assert!(left.len() >= 8);
    debug_assert!(right.len() >= 8);
    // the compiler is smart enough to unroll this loop and use a single xorq on x86_64
    for i in 0..8 {
        left[i] ^= right[i];
    }
}

#[inline]
fn v4_reg_load(dst: &mut u32, src: &[u8]) {
    debug_assert!(src.len() >= 4, "Source must have at least 4 bytes.");
    *dst = u32::from_le_bytes([src[0], src[1], src[2], src[3]]);
}

fn keccak1600(input: &[u8], out: &mut [u8; 200]) {
    let mut hasher = sha3::Keccak256Full::new();
    hasher.write(input).unwrap();
    let result = hasher.finalize();
    out.copy_from_slice(result.as_slice());
}

const NONCE_PTR_INDEX: usize = 35;

fn cn_slow_hash(data: &[u8], variant: Variant, height: u64) -> ([u8; 128], [u8; 16], [u8; 32]) {
    // let long_state: [u8; MEMORY];
    // let text: [u8; INIT_SIZE_BYTE];
    let mut a = [0u8; AES_BLOCK_SIZE];
    let mut b = [0u8; AES_BLOCK_SIZE * 2];
    // let c1: [u8; AES_BLOCK_SIZE];
    // let c2: [u8; AES_BLOCK_SIZE];
    // let d: [u8; AES_BLOCK_SIZE];
    let mut aes_key = [0u8; AES_KEY_SIZE];

    let mut state = CnSlowHashState::default();
    keccak1600(data, &mut state.get_keccak_bytes_mut());
    println!("keccak_state_bytes: {}", hex::encode(state.get_keccak_bytes()));
    aes_key.copy_from_slice(&state.get_k()[0..AES_KEY_SIZE]);
    let aes_expanded_key = cnaes::key_extend(&aes_key);
    let mut text: [u8; INIT_SIZE_BYTE] = state.get_init().try_into().unwrap();
    println!("text(1): {}", hex::encode(&text));
    println!("keccak_state_bytes: {}", hex::encode(state.get_keccak_bytes()));

    // Variant 1 Init
    let mut tweak1_2 = [0u8; 8];
    if variant == Variant::V1 {
        assert!(
            data.len() >= 43,
            "Cryptonight variant 1 needs at least 43 bytes of data"
        );
        tweak1_2.copy_from_slice(state.get_keccak_bytes()[192..200].iter().as_slice());
        xor64(&mut tweak1_2, &data[NONCE_PTR_INDEX..NONCE_PTR_INDEX + 8]);
    }

    // Variant 2 Init
    let mut division_result: u64 = 0;
    let mut sqrt_result: u64 = 0;
    if variant == Variant::V2 || variant == Variant::R {
        let keccak_state_bytes = state.get_keccak_bytes();
        b[AES_BLOCK_SIZE..AES_BLOCK_SIZE + AES_BLOCK_SIZE]
            .copy_from_slice(&keccak_state_bytes[64..64 + AES_BLOCK_SIZE]);
        xor64(&mut b[AES_BLOCK_SIZE..], &keccak_state_bytes[80..]);
        xor64(&mut b[AES_BLOCK_SIZE + 8..], &keccak_state_bytes[88..]);
        division_result = state.get_keccak_word(12);
        sqrt_result = state.get_keccak_word(13);
    }

    // Variant 4 Random Math Init
    let mut r = [0u32; v4::NUM_INSTRUCTIONS_MAX + 1];
    let mut code = [v4::Instruction::default(); v4::NUM_INSTRUCTIONS_MAX + 1];
    let mut keccak_state_bytes = state.get_keccak_bytes();
    if variant == Variant::R {
        for i in 0..4 {
            let j = 12 * 8 + 4 * i;
            r[i] = u32::from_le_bytes(keccak_state_bytes[j..j + 4].try_into().unwrap());
        }
        v4::random_math_init(&mut code, height);
    }

    let mut long_state = vec![0u8; MEMORY];

    for i in 0..MEMORY / INIT_SIZE_BYTE {
        for j in 0..INIT_SIZE_BLK {
            let mut block = &mut text[AES_BLOCK_SIZE * j..AES_BLOCK_SIZE * (j + 1)];
            cnaes::aesb_pseudo_round(&mut block, &aes_expanded_key);
        }

        let start = i * INIT_SIZE_BYTE;
        let end = start + INIT_SIZE_BYTE;
        long_state[start..end].copy_from_slice(&text);
    }

    let k = state.get_k();
    for i in 0..AES_BLOCK_SIZE {
        a[i] = k[i] ^ k[AES_BLOCK_SIZE * 2 + i];
        b[i] = k[AES_BLOCK_SIZE + i] ^ k[AES_BLOCK_SIZE * 3 + i];
    }

    (text, a, b)
}

#[cfg(test)]
mod tests {
    use crate::hash::{cn_slow_hash, Variant};

    #[test]
    fn test_keccak1600() {
        let input = hex::decode("5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374").unwrap();
        let mut output = [0u8; 200];
        super::keccak1600(&input, &mut output);
        let output_hex = "af6fe96f8cb409bdd2a61fb837e346f1a28007b0f078a8d68bc1224b6fcfcc3c39f1244db8c0af06e94173db4a54038a2f7a6a9c729928b5ec79668a30cbf5f266110665e23e891ea4ee2337fb304b35bf8d9c2e4c3524e52e62db67b0b170487a68a34f8026a81b35dc835c60b356d2c411ad227b6c67e30e9b57ba34b3cf27fccecae972850cf3889bb3ff8347b55a5710d58086973d12d75a3340a39430b65ee2f4be27c21e7b39f47341dd036fe13bf43bb2c55bce498a3adcbf07397ea66062b66d56cd8136";
        assert_eq!(hex::encode(output), output_hex);
    }

    #[test]
    fn test_cn_slow_hash() {
        let input = hex::decode("5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374").unwrap();
        let (text, a, b) = cn_slow_hash(&input, Variant::R, 1806260);
        let text_hex = "9f100d9490b133036a6876b3079b17979780ff47e3d1f45e162faef2e8d3df48a72d00fa5a77059b813b0b5ba29c8da4f5de1c52e8bfccb3b7fbaaf489d966fb98fc94ba84f6ee26a5dbb7adcd434d3feeb8c8341787f30fa945fbdbbdc88f68f7958ca68e05d6099d162d13d1060dfe4473a4c1420c662675ec9d1e6b7087c4";
        let a_hex = "969ecd223474a6bb3be76c637db7457b";
        let b_hex = "8dfa6d2c82e1806367b844c15f0439ced99c9a4bae0badfb8a8cf8504b813b7d";
        assert_eq!(hex::encode(text), text_hex);
        assert_eq!(hex::encode(a), a_hex);
        assert_eq!(hex::encode(b), b_hex);
    }
}
