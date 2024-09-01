use crate::hash_v4::variant4_random_math;
use crate::{cnaes, hash_v2 as v2, hash_v4 as v4, subarray, subarray_copy, subarray_mut};
use cnaes::AES_BLOCK_SIZE;
use cnaes::CN_AES_KEY_SIZE;
use digest::Digest;
use groestl::Groestl256;
use jh::Jh256;
use sha3::Digest as _;
use skein::consts::U32;
use skein::Skein512;
use std::cmp::PartialEq;
use std::io::Write;

pub(crate) const MEMORY: usize = 1 << 21; // 2MB scratchpad
const ITER: usize = 1 << 20;
const AES_KEY_SIZE: usize = CN_AES_KEY_SIZE;
const INIT_SIZE_BLK: usize = 8;
const INIT_SIZE_BYTE: usize = INIT_SIZE_BLK * AES_BLOCK_SIZE;

const KECCAK1600_BYTE_SIZE: usize = 200;

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum Variant {
    #[allow(dead_code)]
    V0,
    V1,
    V2,
    R,
}

struct CnSlowHashState {
    b: [u8; KECCAK1600_BYTE_SIZE],
}

impl Default for CnSlowHashState {
    fn default() -> Self {
        CnSlowHashState {
            b: [0; KECCAK1600_BYTE_SIZE],
        }
    }
}

impl CnSlowHashState {
    fn get_keccak_bytes(&self) -> &[u8; KECCAK1600_BYTE_SIZE] {
        &self.b
    }

    fn get_keccak_bytes_mut(&mut self) -> &mut [u8; KECCAK1600_BYTE_SIZE] {
        &mut self.b
    }

    fn get_keccak_word(&self, index: usize) -> u64 {
        u64::from_le_bytes(subarray_copy!(self.b, index * 8, 8))
    }

    fn get_k(&self) -> &[u8; AES_KEY_SIZE * 2] {
        subarray!(self.b, 0, AES_KEY_SIZE * 2)
    }

    fn get_aes_key0(&self) -> &[u8; AES_KEY_SIZE] {
        subarray!(self.b, 0, AES_KEY_SIZE)
    }

    fn get_aes_key1(&self) -> &[u8; AES_KEY_SIZE] {
        subarray!(self.b, AES_KEY_SIZE, AES_KEY_SIZE)
    }

    fn get_init(&self) -> &[u8; INIT_SIZE_BYTE] {
        subarray!(self.b, 64, INIT_SIZE_BYTE)
    }

    fn get_init_mut(&mut self) -> &mut [u8; INIT_SIZE_BYTE] {
        subarray_mut!(self.b, 64, INIT_SIZE_BYTE)
    }
}

/// Performs an XOR operation on the first 8-bytes of two slices placing
/// the result in the 'left' slice.
#[inline]
fn xor64(left: &mut [u8; 8], right: &[u8; 8]) {
    // the compiler is smart enough to unroll this loop and use a single xorq on x86_64
    for i in 0..8 {
        left[i] ^= right[i];
    }
}

fn e2i(a: &[u8; 8], count: usize) -> usize {
    let value: usize = u64::from_le_bytes(*a) as usize;
    (value / AES_BLOCK_SIZE) & (count - 1)
}

fn mul(a: &[u8; 8], b: &[u8; 8], res: &mut [u8; 16]) {
    let a0 = u64::from_le_bytes(*a) as u128;
    let b0 = u64::from_le_bytes(*b) as u128;
    let product = a0.wrapping_mul(b0);
    let hi = (product >> 64) as u64;
    let lo = product as u64;
    // Note: this is a mix of little and big endian below, as high is stored first
    res[0..8].copy_from_slice(&hi.to_le_bytes());
    res[8..16].copy_from_slice(&lo.to_le_bytes());
}

fn sum_half_blocks(a: &mut [u8; 16], b: &[u8; 16]) {
    let a0 = u64::from_le_bytes(subarray_copy!(a, 0, 8));
    let b0 = u64::from_le_bytes(subarray_copy!(b, 0, 8));
    let sum0 = a0.wrapping_add(b0);
    a[0..8].copy_from_slice(&sum0.to_le_bytes());

    let a1 = u64::from_le_bytes(subarray_copy!(a, 8, 8));
    let b1 = u64::from_le_bytes(subarray_copy!(b, 8, 8));
    let sum1 = a1.wrapping_add(b1);
    a[8..16].copy_from_slice(&sum1.to_le_bytes());
}

#[inline]
fn copy_block(dst: &mut [u8; AES_BLOCK_SIZE], src: &[u8; AES_BLOCK_SIZE]) {
    dst.copy_from_slice(src);
}

fn swap_blocks(a: &mut [u8; AES_BLOCK_SIZE], b: &mut [u8; AES_BLOCK_SIZE]) {
    let mut t = [0u8; AES_BLOCK_SIZE];
    t.copy_from_slice(&a[..AES_BLOCK_SIZE]);
    a[..AES_BLOCK_SIZE].copy_from_slice(&b[..AES_BLOCK_SIZE]);
    b[..AES_BLOCK_SIZE].copy_from_slice(&t);
}

fn xor_blocks(a: &mut [u8; AES_BLOCK_SIZE], b: &[u8; AES_BLOCK_SIZE]) {
    for i in 0..AES_BLOCK_SIZE {
        a[i] ^= b[i];
    }
}

fn variant1_1(p11: &mut u8, variant: Variant) {
    if variant == Variant::V1 {
        let tmp = *p11;
        let table = 0x75310u32;
        let index = (((tmp >> 3) & 6) | (tmp & 1)) << 1;
        *p11 = tmp ^ ((table >> index) & 0x30) as u8;
    }
}

fn keccak1600(input: &[u8], out: &mut [u8; KECCAK1600_BYTE_SIZE]) {
    let mut hasher = sha3::Keccak256Full::new();
    _ = hasher.write(input).unwrap();
    let result = hasher.finalize();
    out.copy_from_slice(result.as_slice());
}

const NONCE_PTR_INDEX: usize = 35;

fn hash_permutation(b: &mut [u8; KECCAK1600_BYTE_SIZE]) {
    let mut state = [0u64; 25];

    for (i, chunk) in state.iter_mut().enumerate() {
        *chunk = u64::from_le_bytes(subarray_copy!(b, i * 8, 8));
    }

    keccak::keccak_p(&mut state, 24);

    for (i, chunk) in state.iter().enumerate() {
        subarray_mut!(b, i * 8, 8).copy_from_slice(&chunk.to_le_bytes());
    }
}

fn extra_hashes(input: &[u8; KECCAK1600_BYTE_SIZE], output: &mut [u8; 32]) {
    // Note: the Rust Crypto library only has Blake2, not Blake
    match input[0] & 0x3 {
        0 => blake::hash(256, input, output).unwrap(),
        1 => output.copy_from_slice(Groestl256::digest(input).as_slice()),
        2 => output.copy_from_slice(Jh256::digest(input).as_slice()),
        3 => output.copy_from_slice(Skein512::<U32>::digest(input).as_slice()),
        _ => unreachable!(),
    }
}

pub(crate) fn cn_slow_hash(data: &[u8], variant: Variant, height: u64) -> [u8; 32] {
    let mut b = [0u8; AES_BLOCK_SIZE * 2];

    let mut state = CnSlowHashState::default();
    keccak1600(data, state.get_keccak_bytes_mut());
    let aes_expanded_key = cnaes::key_extend(state.get_aes_key0());
    let mut text = *state.get_init();

    // Variant 1 Init
    let mut tweak1_2 = [0u8; 8];
    if variant == Variant::V1 {
        assert!(
            data.len() >= 43,
            "Cryptonight variant 1 needs at least 43 bytes of data"
        );
        tweak1_2.copy_from_slice(
            state.get_keccak_bytes()[192..KECCAK1600_BYTE_SIZE]
                .iter()
                .as_slice(),
        );
        xor64(&mut tweak1_2, subarray!(data, NONCE_PTR_INDEX, 8));
    }

    // Variant 2 Init
    let mut division_result: u64 = 0;
    let mut sqrt_result: u64 = 0;
    if variant == Variant::V2 || variant == Variant::R {
        let keccak_state_bytes = state.get_keccak_bytes();
        b[AES_BLOCK_SIZE..AES_BLOCK_SIZE + AES_BLOCK_SIZE]
            .copy_from_slice(&keccak_state_bytes[64..64 + AES_BLOCK_SIZE]);
        xor64(
            subarray_mut!(b, AES_BLOCK_SIZE, 8),
            subarray!(keccak_state_bytes, 80, 8),
        );
        xor64(
            subarray_mut!(b, AES_BLOCK_SIZE + 8, 8),
            subarray!(keccak_state_bytes, 88, 8),
        );
        division_result = state.get_keccak_word(12);
        sqrt_result = state.get_keccak_word(13);
    }

    // Variant 4 Random Math Init
    let mut r = [0u32; v4::NUM_INSTRUCTIONS_MAX + 1];
    let mut code = [v4::Instruction::default(); v4::NUM_INSTRUCTIONS_MAX + 1];
    let keccak_state_bytes = state.get_keccak_bytes();
    if variant == Variant::R {
        for i in 0..4 {
            r[i] = u32::from_le_bytes(subarray_copy!(keccak_state_bytes, (24 + i) * 4, 4));
        }
        v4::random_math_init(&mut code, height);
    }

    let mut long_state = vec![0u8; MEMORY]; // use vec to allocate on heap
    let long_state: &mut [u8; MEMORY] = subarray_mut!(long_state, 0, MEMORY);

    for i in 0..MEMORY / INIT_SIZE_BYTE {
        for j in 0..INIT_SIZE_BLK {
            let block = subarray_mut!(text, AES_BLOCK_SIZE * j, AES_BLOCK_SIZE);
            cnaes::aesb_pseudo_round(block, &aes_expanded_key);
        }

        let start = i * INIT_SIZE_BYTE;
        let end = start + INIT_SIZE_BYTE;
        long_state[start..end].copy_from_slice(&text);
    }

    let k = state.get_k();
    let mut a = [0u8; AES_BLOCK_SIZE];
    for i in 0..AES_BLOCK_SIZE {
        a[i] = k[i] ^ k[AES_BLOCK_SIZE * 2 + i];
        b[i] = k[AES_BLOCK_SIZE + i] ^ k[AES_BLOCK_SIZE * 3 + i];
    }

    let mut c1 = [0u8; AES_BLOCK_SIZE];
    let mut c2 = [0u8; AES_BLOCK_SIZE];
    let mut a1 = [0u8; AES_BLOCK_SIZE];

    for _ in 0..ITER / 2 {
        /* Dependency chain: address -> read value ------+
         * written value <-+ hard function (AES or MUL) <+
         * next address  <-+
         */
        // Iteration
        let mut j = e2i(subarray!(a, 0, 8), MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE;
        copy_block(&mut c1, subarray_mut!(long_state, j, AES_BLOCK_SIZE));
        cnaes::aesb_single_round(&mut c1, &a);
        v2::variant2_portable_shuffle_add(&mut c1, &a, &b, long_state, j, variant);

        let long_state_block = subarray_mut!(long_state, j, AES_BLOCK_SIZE);
        copy_block(long_state_block, &c1);
        xor_blocks(long_state_block, subarray!(b, 0, AES_BLOCK_SIZE));
        variant1_1(&mut long_state_block[11], variant);

        /* Iteration 2 */
        j = e2i(subarray!(c1, 0, 8), MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE;
        let long_state_block = subarray_mut!(long_state, j, AES_BLOCK_SIZE);
        copy_block(&mut c2, long_state_block);
        copy_block(&mut a1, &a);
        v2::variant2_integer_math(
            subarray_mut!(c2, 0, 8),
            &c1,
            &mut division_result,
            &mut sqrt_result,
            variant,
        );
        variant4_random_math(
            &mut a1,
            &mut c2,
            (&mut r[0..9]).try_into().unwrap(),
            &b,
            &code,
        );
        let mut d = [0u8; AES_BLOCK_SIZE]; // TODO: have mul return d
        mul(subarray!(c1, 0, 8), subarray!(c2, 0, 8), &mut d);

        // VARIANT2_2_PORTABLE
        // TODO: move some of this code to hash_v2???
        if variant == Variant::V2 {
            let chunk1_start = j ^ 0x10;
            let chunk2_start = j ^ 0x20;
            xor_blocks(subarray_mut!(long_state, chunk1_start, AES_BLOCK_SIZE), &d);
            xor_blocks(
                &mut d,
                subarray_mut!(long_state, chunk2_start, AES_BLOCK_SIZE),
            );
        }

        v2::variant2_portable_shuffle_add(&mut c1, &a, &b, long_state, j, variant);

        sum_half_blocks(&mut a1, &d);
        swap_blocks(&mut a1, &mut c2);
        xor_blocks(&mut a1, &c2);

        // VARIANT1_2
        if variant == Variant::V1 {
            xor64(subarray_mut!(c2, 8, 8), &tweak1_2);
        }
        copy_block(subarray_mut!(long_state, j, AES_BLOCK_SIZE), &c2);
        if variant == Variant::V2 || variant == Variant::R {
            let (b_half1, b_half2) = b.split_at_mut(AES_BLOCK_SIZE);
            copy_block(
                subarray_mut!(b_half2, 0, AES_BLOCK_SIZE),
                subarray!(b_half1, 0, AES_BLOCK_SIZE),
            );
        }
        copy_block(subarray_mut!(b, 0, AES_BLOCK_SIZE), &c1);
        copy_block(&mut a, &a1);
    }

    text.copy_from_slice(state.get_init());
    let aes_expanded_key = cnaes::key_extend(state.get_aes_key1());
    for i in 0..MEMORY / INIT_SIZE_BYTE {
        for j in 0..INIT_SIZE_BLK {
            let block = subarray_mut!(text, AES_BLOCK_SIZE * j, AES_BLOCK_SIZE);
            let ls_index = i * INIT_SIZE_BYTE + j * AES_BLOCK_SIZE;
            xor_blocks(block, subarray!(long_state, ls_index, AES_BLOCK_SIZE));
            cnaes::aesb_pseudo_round(block, &aes_expanded_key);
        }
    }
    state.get_init_mut().copy_from_slice(&text);

    hash_permutation(state.get_keccak_bytes_mut());

    let mut hash = [0u8; 32];
    extra_hashes(state.get_keccak_bytes(), &mut hash);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::hex_to_array;
    use groestl::{Digest, Groestl256};

    #[test]
    fn test_keccak1600() {
        let input: [u8; 44] = hex_to_array(
            "5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374"
        );
        let mut output = [0u8; KECCAK1600_BYTE_SIZE];
        super::keccak1600(&input, &mut output);
        let output_hex = "af6fe96f8cb409bdd2a61fb837e346f1a28007b0f078a8d68bc1224b6fcfcc3c39f1244db8c0af06e94173db4a54038a2f7a6a9c729928b5ec79668a30cbf5f266110665e23e891ea4ee2337fb304b35bf8d9c2e4c3524e52e62db67b0b170487a68a34f8026a81b35dc835c60b356d2c411ad227b6c67e30e9b57ba34b3cf27fccecae972850cf3889bb3ff8347b55a5710d58086973d12d75a3340a39430b65ee2f4be27c21e7b39f47341dd036fe13bf43bb2c55bce498a3adcbf07397ea66062b66d56cd8136";
        assert_eq!(hex::encode(output), output_hex);
    }

    #[test]
    fn test_mul() {
        let test = |a_hex: &str, b_hex: &str, expected_hex: &str| {
            let a: [u8; 8] = hex_to_array(a_hex);
            let b: [u8; 8] = hex_to_array(b_hex);
            let mut res = [0u8; 16];
            super::mul(&a, &b, &mut res);
            assert_eq!(hex::encode(res), expected_hex);
        };
        test(
            "0100000000000000",
            "0100000000000000",
            "00000000000000000100000000000000",
        );
        test(
            "ffffffffffffffff",
            "0200000000000000",
            "0100000000000000feffffffffffffff",
        );
        test(
            "34504affdab54e6d",
            "b352de34917bcc4f",
            "2d82d3509a9912225cbcbe6b16321e17",
        );
        test(
            "26ce23ce804055ed",
            "d8e42f12da72202a",
            "1f531a54b7110e2710c8c956b3f98f90",
        );
    }

    #[test]
    fn test_groestl256() {
        let input: [u8; 32] =
            hex_to_array("f759588ad57e758467295443a9bd71490abff8e9dad1b95b6bf2f5d0d78387bc");
        let mut output = [0u8; 32];
        output.copy_from_slice(Groestl256::digest(&input).as_slice());
        let expected_hex = "3085f5b0f7126a1d10e6da550ee44c51f0fcad91a80e78268ca5669f0bff0a4e";
        assert_eq!(hex::encode(output), expected_hex);
    }

    #[test]
    fn test_cn_slow_hash() {
        // TODO: Add a full test vector here
        let input: [u8; 44] = hex_to_array("5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374");
        const EXPECTED: &str = "f759588ad57e758467295443a9bd71490abff8e9dad1b95b6bf2f5d0d78387bc";
        let hash = cn_slow_hash(&input, Variant::R, 1806260);
        assert_eq!(hex::encode(hash), EXPECTED);
    }

    #[test]
    fn test_hash_permutations() {
        let mut state_bytes: [u8; KECCAK1600_BYTE_SIZE] = hex_to_array(
            "af6fe96f8cb409bdd2a61fb837e346f1a28007b0f078a8d68bc1224b6fcfcc3c39f1244db8c0af06e94173db4a54038a2f7a6a9c729928b5ec79668a30cbf5f2622fea9d7982e587e6612c4e6a1d28fdbaba4af1aea99e63322a632d514f35b4fc5cf231e9a6328efb5eb22ad2cfabe571ee8b6ef7dbc64f63185d54a771bdccd207b75e10547b4928f5dcb309192d88bf313d8bc53c8fe71da7ea93355d266c5cc8d39a1273e44b074d143849a3b302edad73c2e61f936c502f6bbabb972b616062b66d56cd8136"
        );
        const EXPECTED: &str = "31e2fb6eb8e2e376d42a53bc88166378f2a23cf9be54645ff69e8ade3aa4b7ad35040d0e3ad0ee0d8562d53a51acdf14f44de5c097c48a29f63676346194b3af13c3c45af214335a14329491081068a32ea29b3a6856e0efa737dff49d3b5dbf3f7847f058bb41d36347c19d5cd5bdb354ac64a86156c8194e19b0f62d109a8112024a7734730a2bb221c137d3034204e1e57d9cec9689bc199de684f38aeed4624b84c39675a4755ce9b69fde9d36cabd12f1aef4a5b2bb6c6126900799f2109e9b6b55d7bb3ff5";
        super::hash_permutation(&mut state_bytes);
        assert_eq!(hex::encode(state_bytes), EXPECTED);
    }

    #[test]
    fn test_extra_hashes() {
        let mut input = [0u8; KECCAK1600_BYTE_SIZE];
        for i in 0..input.len() {
            input[i] = i as u8;
        }

        let mut output = [0u8; 32];

        const EXPECTED_BLAKE: &str =
            "c4d944c2b1c00a8ee627726b35d4cd7fe018de090bc637553cc782e25f974cba";
        const EXPECTED_GROESTL: &str =
            "73905cfed57520c60eb468defc58a925170cecc6b4a9f2f6e56d34d674d64111";
        const EXPECTED_JH: &str =
            "71a4f8ae96c48df7ace370854824a60a2f247fbf903c7b936f6f99d164c2f6b1";
        const EXPECTED_SKEIN: &str =
            "040e79b9daa0fc6219234a06b3889f86f8b02b78dcc25a9874ca95630cf6b5e6";

        const EXPECTED: [&str; 4] = [
            EXPECTED_BLAKE,
            EXPECTED_GROESTL,
            EXPECTED_JH,
            EXPECTED_SKEIN,
        ];

        for (i, expected) in EXPECTED.iter().enumerate() {
            input[0] = i as u8;
            extra_hashes(&input, &mut output);
            assert_eq!(hex::encode(&output), *expected, "hash {}", i);
        }
    }
}
