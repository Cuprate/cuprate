use crate::hash_v4::variant4_random_math;
use crate::{cnaes, hash_v4 as v4};
use digest::Digest;
use groestl::Groestl256;
use jh::Jh256;
use sha3::Digest as _;
use skein::consts::U32;
use skein::Skein512;
use std::cmp::PartialEq;
use std::io::Write;
use std::u64;

const MEMORY: usize = 1 << 21; // 2MB scratchpad
const ITER: usize = 1 << 20;
const AES_BLOCK_SIZE: usize = 16;
const AES_KEY_SIZE: usize = 32;
const INIT_SIZE_BLK: usize = 8;
const INIT_SIZE_BYTE: usize = INIT_SIZE_BLK * AES_BLOCK_SIZE;

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum Variant {
    #[allow(dead_code)]
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
        CnSlowHashState { b: [0; 200] }
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

    fn get_k(&mut self) -> [u8; 64] {
        self.b[0..64].try_into().unwrap()
    }

    // fn get_k_mut(&mut self) -> &mut [u8; 64] {
    //     (&mut self.b[0..64]).try_into().unwrap()
    // }

    fn get_init(&self) -> [u8; INIT_SIZE_BYTE] {
        self.b[64..64 + INIT_SIZE_BYTE].try_into().unwrap()
    }

    fn get_init_mut(&mut self) -> &mut [u8; INIT_SIZE_BYTE] {
        (&mut self.b[64..64 + INIT_SIZE_BYTE]).try_into().unwrap()
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

fn e2i(a: &[u8], count: usize) -> usize {
    let value: usize = u64::from_le_bytes(a[..8].try_into().unwrap()) as usize;
    (value / AES_BLOCK_SIZE) & (count - 1)
}

fn mul(a: &[u8; 8], b: &[u8; 8], res: &mut [u8; 16]) {
    let a0 = u64::from_le_bytes(a[0..8].try_into().unwrap()) as u128;
    let b0 = u64::from_le_bytes(b[0..8].try_into().unwrap()) as u128;
    let product = a0.wrapping_mul(b0);
    let hi = (product >> 64) as u64;
    let lo = product as u64;
    res[0..8].copy_from_slice(&hi.to_le_bytes());
    res[8..16].copy_from_slice(&lo.to_le_bytes());
}

fn sum_half_blocks(a: &mut [u8], b: &[u8]) {
    assert!(a.len() >= 16);
    assert!(b.len() >= 16);

    let a0 = u64::from_le_bytes(a[0..8].try_into().unwrap());
    let a1 = u64::from_le_bytes(a[8..16].try_into().unwrap());
    let b0 = u64::from_le_bytes(b[0..8].try_into().unwrap());
    let b1 = u64::from_le_bytes(b[8..16].try_into().unwrap());

    let a0 = a0.wrapping_add(b0);
    let a1 = a1.wrapping_add(b1);

    a[0..8].copy_from_slice(&a0.to_le_bytes());
    a[8..16].copy_from_slice(&a1.to_le_bytes());
}

fn copy_block(dst: &mut [u8], src: &[u8]) {
    assert!(dst.len() >= AES_BLOCK_SIZE);
    assert!(src.len() >= AES_BLOCK_SIZE);
    dst[..AES_BLOCK_SIZE].copy_from_slice(&src[..AES_BLOCK_SIZE]);
}

fn swap_blocks(a: &mut [u8], b: &mut [u8]) {
    assert!(a.len() >= AES_BLOCK_SIZE);
    assert!(b.len() >= AES_BLOCK_SIZE);
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

fn variant2_portable_shuffle_add(
    c1: &mut [u8; AES_BLOCK_SIZE],
    a: &[u8; AES_BLOCK_SIZE],
    b: &[u8; AES_BLOCK_SIZE * 2],
    long_state: &mut [u8; MEMORY],
    offset: usize,
    variant: Variant,
) {
    if variant == Variant::V2 || variant == Variant::R {
        let chunk1_start = offset ^ 0x10;
        let chunk2_start = offset ^ 0x20;
        let chunk3_start = offset ^ 0x30;

        let chunk1 = &long_state[chunk1_start..chunk1_start + 16];
        let chunk2 = &long_state[chunk2_start..chunk2_start + 16];
        let chunk3 = &long_state[chunk3_start..chunk3_start + 16];

        let mut chunk1_old = [
            u64::from_le_bytes(chunk1[0..8].try_into().unwrap()),
            u64::from_le_bytes(chunk1[8..16].try_into().unwrap()),
        ];
        let chunk2_old = [
            u64::from_le_bytes(chunk2[0..8].try_into().unwrap()),
            u64::from_le_bytes(chunk2[8..16].try_into().unwrap()),
        ];
        let chunk3_old = [
            u64::from_le_bytes(chunk3[0..8].try_into().unwrap()),
            u64::from_le_bytes(chunk3[8..16].try_into().unwrap()),
        ];

        let b1 = [
            u64::from_le_bytes(b[16..24].try_into().unwrap()),
            u64::from_le_bytes(b[24..32].try_into().unwrap()),
        ];
        let chunk1 = &mut long_state[chunk1_start..chunk1_start + 16];
        chunk1[0..8].copy_from_slice(&(chunk3_old[0].wrapping_add(b1[0]).to_le_bytes()));
        chunk1[8..16].copy_from_slice(&(chunk3_old[1].wrapping_add(b1[1]).to_le_bytes()));

        let a0 = [
            u64::from_le_bytes(a[0..8].try_into().unwrap()),
            u64::from_le_bytes(a[8..16].try_into().unwrap()),
        ];

        let chunk3 = &mut long_state[chunk3_start..chunk3_start + 16];
        chunk3[0..8].copy_from_slice(&(chunk2_old[0].wrapping_add(a0[0])).to_le_bytes());
        chunk3[8..16].copy_from_slice(&(chunk2_old[1].wrapping_add(a0[1])).to_le_bytes());

        let b0 = [
            u64::from_le_bytes(b[0..8].try_into().unwrap()),
            u64::from_le_bytes(b[8..16].try_into().unwrap()),
        ];
        let chunk2 = &mut long_state[chunk2_start..chunk2_start + 16];
        chunk2[0..8].copy_from_slice(&(chunk1_old[0].wrapping_add(b0[0])).to_le_bytes());
        chunk2[8..16].copy_from_slice(&(chunk1_old[1].wrapping_add(b0[1])).to_le_bytes());

        if variant == Variant::R {
            let mut out_copy = [
                u64::from_le_bytes(c1[0..8].try_into().unwrap()),
                u64::from_le_bytes(c1[8..16].try_into().unwrap()),
            ];

            chunk1_old[0] ^= chunk2_old[0];
            chunk1_old[1] ^= chunk2_old[1];
            out_copy[0] ^= chunk3_old[0];
            out_copy[1] ^= chunk3_old[1];
            out_copy[0] ^= chunk1_old[0];
            out_copy[1] ^= chunk1_old[1];

            c1[0..8].copy_from_slice(&out_copy[0].to_le_bytes());
            c1[8..16].copy_from_slice(&out_copy[1].to_le_bytes());
        }
    }
}

fn variant2_integer_math_sqrt(sqrt_input: u64) -> u64 {
    // Get an approximation using floating point math
    let mut sqrt_result =
        ((sqrt_input as f64 + 18446744073709551616.0).sqrt() * 2.0 - 8589934592.0) as u64;

    // Fixup the edge cases to get the exact integer result. For more information,
    // see: https://github.com/monero-project/monero/blob/v0.18.3.3/src/crypto/variant2_int_sqrt.h#L65-L152
    let sqrt_div2 = sqrt_result >> 1;
    let lsb = sqrt_result & 1;
    let r2 = sqrt_div2
        .wrapping_mul(sqrt_div2 + lsb)
        .wrapping_add(sqrt_result << 32);

    if r2.wrapping_add(lsb) > sqrt_input {
        sqrt_result = sqrt_result.wrapping_sub(1);
    }
    if r2.wrapping_add(1 << 32) < sqrt_input.wrapping_sub(sqrt_div2) {
        // Not sure that this is possible. I tried writing a test program
        // to search subsets of u64 for a value that can trigger this
        // branch, but couldn't find anything. The Go implementation came
        // to the same conclusion:
        // https://github.com/Equim-chan/cryptonight/blob/v0.3.0/arith_ref.go#L39-L45
        sqrt_result = sqrt_result.wrapping_add(1);
    }

    sqrt_result
}

fn variant2_integer_math(
    c1: &mut [u8; 16],
    c2: &[u8; 16],
    division_result: &mut u64,
    sqrt_result: &mut u64,
    variant: Variant,
) {
    const U32_MASK: u64 = u32::MAX as u64;

    if variant == Variant::V2 {
        let tmpx = *division_result ^ (*sqrt_result << 32);
        let c1_0 = u64::from_le_bytes(c1[0..8].try_into().unwrap());
        let c1_0 = c1_0 ^ tmpx;
        c1[0..8].copy_from_slice(&c1_0.to_le_bytes());

        let dividend = u64::from_le_bytes(c2[8..16].try_into().unwrap());
        let mut divisor = u64::from_le_bytes(c2[0..8].try_into().unwrap());
        divisor = ((divisor + ((*sqrt_result << 1) & U32_MASK)) | 0x80000001) & U32_MASK;
        *division_result =
            ((dividend / divisor) & U32_MASK).wrapping_add((dividend % divisor) << 32);

        let sqrt_input =
            u64::from_le_bytes(c2[0..8].try_into().unwrap()).wrapping_add(*division_result);
        *sqrt_result = variant2_integer_math_sqrt(sqrt_input);
    }
}

fn variant1_1(p: &mut [u8], variant: Variant) {
    if variant == Variant::V1 {
        let tmp = p[11];
        let table = 0x75310u32;
        let index = (((tmp >> 3) & 6) | (tmp & 1)) << 1;
        p[11] = tmp ^ ((table >> index) & 0x30) as u8;
    }
}

fn keccak1600(input: &[u8], out: &mut [u8; 200]) {
    let mut hasher = sha3::Keccak256Full::new();
    hasher.write(input).unwrap();
    let result = hasher.finalize();
    out.copy_from_slice(result.as_slice());
}

const NONCE_PTR_INDEX: usize = 35;

// fn hashed(data: &[u8]) -> String {
//     let mut output = [0u8; 32];
//     blake::hash(256, data, &mut output).expect("blake hash failed");
//     hex::encode(output)
// }

fn hash_permutation(b: &mut [u8; 200]) {
    let mut state = [0u64; 25];

    for (i, chunk) in state.iter_mut().enumerate() {
        *chunk = u64::from_le_bytes(b[i * 8..(i + 1) * 8].try_into().unwrap());
    }

    keccak::keccak_p(&mut state, 24);

    for (i, chunk) in state.iter().enumerate() {
        b[i * 8..(i + 1) * 8].copy_from_slice(&chunk.to_le_bytes());
    }
}

fn extra_hashes(input: &[u8; 200], output: &mut [u8; 32]) {
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
    let mut aes_key = [0u8; AES_KEY_SIZE];

    let mut state = CnSlowHashState::default();
    keccak1600(data, &mut state.get_keccak_bytes_mut());
    aes_key.copy_from_slice(&state.get_k()[0..AES_KEY_SIZE]);
    let aes_expanded_key = cnaes::key_extend(&aes_key);
    let mut text: [u8; INIT_SIZE_BYTE] = state.get_init();

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
    let keccak_state_bytes = state.get_keccak_bytes();
    if variant == Variant::R {
        for i in 0..4 {
            let j = 12 * 8 + 4 * i;
            r[i] = u32::from_le_bytes(keccak_state_bytes[j..j + 4].try_into().unwrap());
        }
        v4::random_math_init(&mut code, height);
    }

    let mut long_state = vec![0u8; MEMORY]; // use vec to allocate on heap
    let mut long_state: &mut [u8; MEMORY] = (&mut long_state[..]).try_into().unwrap();

    for i in 0..MEMORY / INIT_SIZE_BYTE {
        for j in 0..INIT_SIZE_BLK {
            let block: &mut [u8; AES_BLOCK_SIZE] = (&mut text
                [AES_BLOCK_SIZE * j..AES_BLOCK_SIZE * (j + 1)])
                .try_into()
                .unwrap();
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
        let mut j = e2i(&a[..8], MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE;
        copy_block(&mut c1[..], &long_state[j..j + AES_BLOCK_SIZE]);
        cnaes::aesb_single_round(&mut c1, &a);
        variant2_portable_shuffle_add(&mut c1, &a, &b, &mut long_state, j, variant);

        copy_block(
            &mut long_state[j..j + AES_BLOCK_SIZE],
            &c1[..AES_BLOCK_SIZE],
        );
        xor_blocks(
            (&mut long_state[j..j + AES_BLOCK_SIZE]).try_into().unwrap(),
            (&b[..AES_BLOCK_SIZE]).try_into().unwrap(),
        );
        assert_eq!(j, e2i(&a[..8], MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE);
        variant1_1(&mut long_state[j..], variant);

        /* Iteration 2 */
        j = e2i(&c1, MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE;
        copy_block(&mut c2, &long_state[j..]);
        copy_block(&mut a1, &a);
        variant2_integer_math(
            &mut c2,
            &c1,
            &mut division_result,
            &mut sqrt_result,
            variant,
        );
        variant4_random_math(
            &mut a1,
            &mut c2,
            (&mut r[0..9]).try_into().unwrap(),
            &mut b,
            &code,
        );
        let mut d = [0u8; AES_BLOCK_SIZE]; // TODO: have mul return d
        mul(
            &c1[0..8].try_into().unwrap(),
            &c2[0..8].try_into().unwrap(),
            &mut d,
        );

        // VARIANT2_2_PORTABLE
        if variant == Variant::V2 {
            let chunk1_start = j ^ 0x10;
            let chunk2_start = j ^ 0x20;
            xor_blocks(
                (&mut long_state[chunk1_start..chunk1_start + AES_BLOCK_SIZE])
                    .try_into()
                    .unwrap(),
                &d,
            );
            xor_blocks(
                &mut d,
                (&long_state[chunk2_start..chunk2_start + AES_BLOCK_SIZE])
                    .try_into()
                    .unwrap(),
            );
        }

        variant2_portable_shuffle_add(&mut c1, &a, &b, &mut long_state, j, variant);

        sum_half_blocks(&mut a1, &d);
        swap_blocks(&mut a1, &mut c2);
        xor_blocks(&mut a1, &mut c2);

        // VARIANT1_2
        if variant == Variant::V1 {
            xor64(&mut c2[8..16], &tweak1_2);
        }
        copy_block(&mut long_state[j..], &c2);
        if variant == Variant::V2 || variant == Variant::R {
            let (b_start, mut b_rest) = b.split_at_mut(AES_BLOCK_SIZE);
            copy_block(&mut b_rest, &b_start);
        }
        copy_block(&mut b, &c1);
        copy_block(&mut a, &a1);
    }

    text.copy_from_slice(&state.get_init());
    aes_key.copy_from_slice(&state.get_k()[AES_KEY_SIZE..]);
    let aes_expanded_key = cnaes::key_extend(&aes_key);
    for i in 0..MEMORY / INIT_SIZE_BYTE {
        for j in 0..INIT_SIZE_BLK {
            let mut block: &mut [u8; AES_BLOCK_SIZE] = (&mut text
                [AES_BLOCK_SIZE * j..AES_BLOCK_SIZE * (j + 1)])
                .try_into()
                .unwrap();
            let ls_index = i * INIT_SIZE_BYTE + j * AES_BLOCK_SIZE;
            xor_blocks(
                &mut block,
                (&long_state[ls_index..ls_index + AES_BLOCK_SIZE])
                    .try_into()
                    .unwrap(),
            );
            cnaes::aesb_pseudo_round(&mut block, &aes_expanded_key);
        }
    }
    state.get_init_mut().copy_from_slice(&text);

    hash_permutation(&mut state.get_keccak_bytes_mut());

    let mut hash = [0u8; 32];
    extra_hashes(state.get_keccak_bytes(), &mut hash);
    hash
}

#[cfg(test)]
mod tests {
    use crate::hash::{
        cn_slow_hash, extra_hashes, variant2_integer_math_sqrt, Variant, AES_BLOCK_SIZE, MEMORY,
    };
    use groestl::digest::typenum::U32;
    use groestl::{Digest, Groestl256};
    use hex_literal::hex;
    use keccak;

    #[test]
    fn test_keccak1600() {
        let input = hex!(
            "5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374"
        );
        let mut output = [0u8; 200];
        super::keccak1600(&input, &mut output);
        let output_hex = "af6fe96f8cb409bdd2a61fb837e346f1a28007b0f078a8d68bc1224b6fcfcc3c39f1244db8c0af06e94173db4a54038a2f7a6a9c729928b5ec79668a30cbf5f266110665e23e891ea4ee2337fb304b35bf8d9c2e4c3524e52e62db67b0b170487a68a34f8026a81b35dc835c60b356d2c411ad227b6c67e30e9b57ba34b3cf27fccecae972850cf3889bb3ff8347b55a5710d58086973d12d75a3340a39430b65ee2f4be27c21e7b39f47341dd036fe13bf43bb2c55bce498a3adcbf07397ea66062b66d56cd8136";
        assert_eq!(hex::encode(output), output_hex);
    }

    #[test]
    fn test_mul() {
        let test = |a_hex: &str, b_hex: &str, expected_hex: &str| {
            let a: [u8; 8] = hex::decode(a_hex).unwrap().try_into().unwrap();
            let b: [u8; 8] = hex::decode(b_hex).unwrap().try_into().unwrap();
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
    fn test_variant2_integer_math_sqrt() {
        // Edge case values taken from here:
        // https://github.com/monero-project/monero/blob/v0.18.3.3/src/crypto/variant2_int_sqrt.h#L33-L43
        let test_cases = [
            (0, 0),
            (1 << 32, 0),
            ((1 << 32) + 1, 1),
            (1 << 50, 262140),
            ((1 << 55) + 20963331, 8384515),
            ((1 << 55) + 20963332, 8384516),
            ((1 << 62) + 26599786, 1013904242),
            ((1 << 62) + 26599787, 1013904243),
            (u64::MAX, 3558067407),
        ];

        for &(input, expected) in &test_cases {
            assert_eq!(
                variant2_integer_math_sqrt(input),
                expected,
                "input = {}",
                input
            );
        }
    }

    #[test]
    fn test_variant2_integer_math() {
        let test = |c2_hex: &str,
                    c1_hex: &str,
                    division_result: u64,
                    sqrt_result: u64,
                    c2_hex_end: &str,
                    division_result_end: u64,
                    sqrt_result_end: u64| {
            let mut c2: [u8; 16] = hex::decode(c2_hex).unwrap().try_into().unwrap();
            let c1: [u8; 16] = hex::decode(c1_hex).unwrap().try_into().unwrap();
            let mut division_result = division_result;
            let mut sqrt_result = sqrt_result;
            super::variant2_integer_math(
                &mut c2,
                &c1,
                &mut division_result,
                &mut sqrt_result,
                Variant::V2,
            );
            assert_eq!(hex::encode(c2), c2_hex_end);
            assert_eq!(division_result, division_result_end);
            assert_eq!(sqrt_result, sqrt_result_end);
        };
        test(
            "8b4d610801fe2049741c4cf1a11912d5",
            "ef9d5925ad73f044f6310bce80f333a4",
            1992885167645223034,
            15156498822412360757,
            "f125c247b4040b0e741c4cf1a11912d5",
            11701596267494179432,
            3261805857,
        );
        test(
            "540ac7dbbddf5b93fdc90f999408b7ad",
            "10d2c1fdcbf7246e8623a3d946bdf422",
            6226440187041759132,
            1708636566,
            "c83510b077a4e4a0fdc90f999408b7ad",
            6478148604080708997,
            2875078897,
        );
        test(
            "0df28c3c3570ae3b68dc9d6c5a486ed7",
            "a5fba99aa63fa032acf1bd65ff4df3f2",
            11107069037757228366,
            2924318811,
            "4397ce171fdcc70f68dc9d6c5a486ed7",
            7549089838000449301,
            2299293038,
        );
        test(
            "bfe14f97a968a35d0dcd6890a03c2913",
            "d4a80e16ad64e3a0624a795c7b349c8a",
            15584044376391133794,
            276486141,
            "dd4bf8759e1a9c950dcd6890a03c2913",
            4771913259875991617,
            3210383690,
        );

        test(
            "820692e47779a9aabf0621e52a142468",
            "df61b75f65251ee61828166e565336a9",
            3269677112081011360,
            1493829760,
            "2254426ff54bc3debf0621e52a142468",
            2626216843989114230,
            175440206,
        );
        test(
            "0b364e61de218e00e83c4073b39daa2e",
            "cc463d4543eb430d08efedf2be86e322",
            7096668609104405526,
            713261042,
            "1d521b6fac307148e83c4073b39daa2e",
            8234613052379859783,
            1924288792,
        );
        test(
            "bd8fff861f6315c2be812b64cbdcf646",
            "38d1e323d9dc282fa5e68f2ecbdcb950",
            9545374795048279136,
            271106137,
            "dd532ef48b584a56be812b64cbdcf646",
            2790373411402251888,
            1336862722,
        );
        test(
            "ed57e73448f357bf04dc831d5e8fd848",
            "a5dcd0971e6ded60d4d98c03cd8ba205",
            5991074580974163125,
            2246952057,
            "580331e9a7a59e6904dc831d5e8fd848",
            7395390641079862703,
            2868947253,
        );
        test(
            "07ea0ffc6e182a7e97853f82e459d625",
            "7e403d950f4adc97b90140875c33d65f",
            8836830558353968711,
            1962375668,
            "40a40e3f08db7f7097853f82e459d625",
            5478469695216926448,
            3219877666,
        );
        test(
            "b77688d600a356077021e2333ee3def4",
            "7a9f061760287a69b57f365163fb9dac",
            3127636279441542418,
            1585025819,
            "a5bb34d8bba848727021e2333ee3def4",
            3683326568856788118,
            2315202244,
        );
        test(
            "a246a7f62b7e3d9a0b5ac66166bfcba3",
            "23329476afdbd46d3be9d3ccc9011c11",
            12123559059253265496,
            819016365,
            "fac2e5d23dc4d3020b5ac66166bfcba3",
            4214751652441358299,
            2469122821,
        );
        test(
            "3e1abb8109c688405cd6c866cbdb3e13",
            "b4c10bf5e06c069928afa173f62d5017",
            7368515032603121941,
            2312559799,
            "2b43d451df231caf5cd6c866cbdb3e13",
            1324536149240623108,
            2509236669,
        );
        test(
            "a31260db7c73f249b5fbc182ae7fcc8e",
            "b4214755b0003e4c82d03f80d8a06bed",
            1904095218141907119,
            92928147,
            "0c5abeec6c3f1756b5fbc182ae7fcc8e",
            9883090335304272258,
            3041688469,
        );
        test(
            "e3d0bc3e619f577a1eea5adba205e494",
            "cd8040848aae39104c310c1fa0eed9b8",
            4873400164336079541,
            2436984787,
            "56c22935133bb7a81eea5adba205e494",
            8226478499779865232,
            1963241245,
        );
        test(
            "f22ac244fd17cf5e3ec21bece2581a2d",
            "785152f272ffa9514ef2ae0bed5cbaa7",
            6386228481616770937,
            1413583152,
            "8bddfda13af62e523ec21bece2581a2d",
            9654977853452823978,
            3069608655,
        );
        test(
            "37b3921988d9df1b38b04dc1db01a41b",
            "054b87f38d203eddb16d458048f3b97b",
            5592059432235016971,
            2670380708,
            "3c10afec40e36fc938b04dc1db01a41b",
            2475375116655310772,
            3553266751,
        );
        test(
            "cfd4afb021e526d9cbd4720cc47c4ce2",
            "a2e3e7fe936c2b38e3708965f2dfc586",
            11958325643570725319,
            825185219,
            "0895d52d3237fd4dcbd4720cc47c4ce2",
            2253955666499039951,
            1359567468,
        );
        test(
            "55d2ea9570994bc0aeaf6a3189bf0b4a",
            "9d102c34665382dfd36e39a67e07b8aa",
            10171590341391886242,
            541577843,
            "f7f59fbe85f4246daeaf6a3189bf0b4a",
            6907584596503955220,
            1004462004,
        );
        test(
            "bf32b60d6bbaa87cececd577f2ad15d8",
            "9a8471b2b72e9d39cd2d2cb124aa270a",
            9778648685358392468,
            469385479,
            "2b9696774746e6e0ececd577f2ad15d8",
            4910280747850874346,
            1899784302,
        );
        test(
            "d70ac5de7a390e2a735726324d0b52b5",
            "6cf5b75b005599047972995ffbe34101",
            2318211298357120319,
            1093372020,
            "e8871a66ea410e4b735726324d0b52b5",
            14587709575956469579,
            2962700286,
        );
        test(
            "412f463e5143eace451dcb2a2efd8022",
            "38ed251c7915236b2aca4ea995b861c9",
            10458537212399393571,
            621387691,
            "623403e9d4ecc77a451dcb2a2efd8022",
            12914179687381327414,
            495045866,
        );
    }

    #[test]
    fn test_groestl256() {
        let input: [u8; 32] =
            hex!("f759588ad57e758467295443a9bd71490abff8e9dad1b95b6bf2f5d0d78387bc");
        let mut output = [0u8; 32];
        output.copy_from_slice(Groestl256::digest(&input).as_slice());
        let expected_hex = "3085f5b0f7126a1d10e6da550ee44c51f0fcad91a80e78268ca5669f0bff0a4e";
        assert_eq!(hex::encode(output), expected_hex);
    }

    #[test]
    fn test_variant2_portable_shuffle_add() {
        let test = |c1_hex: &str,
                    a_hex: &str,
                    b_hex: &str,
                    offset: usize,
                    variant: Variant,
                    c1_hex_end: &str,
                    long_state_end_hash: &str| {
            let mut c1: [u8; AES_BLOCK_SIZE] =
                hex::decode(c1_hex).unwrap().as_slice().try_into().unwrap();
            let a: [u8; AES_BLOCK_SIZE] =
                hex::decode(a_hex).unwrap().as_slice().try_into().unwrap();
            let b: [u8; AES_BLOCK_SIZE * 2] =
                hex::decode(b_hex).unwrap().as_slice().try_into().unwrap();

            let mut long_state = Box::new([0u8; MEMORY]);
            for (i, byte) in long_state.iter_mut().enumerate() {
                *byte = i as u8;
            }

            super::variant2_portable_shuffle_add(
                &mut c1,
                &a[0..16].try_into().unwrap(),
                &b[0..32].try_into().unwrap(),
                &mut long_state,
                offset,
                variant,
            );
            assert_eq!(hex::encode(c1), c1_hex_end);
            let hash = Groestl256::digest(long_state.as_slice());
            assert_eq!(hex::encode(hash), long_state_end_hash);
        };
        test(
            "d7143e3b6ffdeae4b2ceea30e9889c8a",
            "875fa34de3af48f15638bad52581ef4c",
            "b07d6f24f19434289b305525f094d8d7bd9d3c9bc956ac081d6186432a282a36",
            221056,
            Variant::R,
            "5795bcb8eb786c633a4760bb65051205",
            "26c32c4c2eeec340d62b88f5261d1a264c74240c2f8424c6e7101cf490e5772e",
        );
        test(
            "c7d6fe95ffd8d902d2cfc1883f7a2bc3",
            "bceb9d8cb71c2ac85c24129c94708e17",
            "4b3a589c187e26bea487b19ea36eb19e8369f4825642eb467c75bf07466b87ba",
            1960880,
            Variant::V2,
            "c7d6fe95ffd8d902d2cfc1883f7a2bc3",
            "2d4ddadd0e53a02797c62bf37d11bb2de73e6769abd834a81c1262752176a024",
        );
        test(
            "92ad41fc1596244e2e0f0bfed6555cef",
            "d1f0337e48c4f53742cedd78b6b33b67",
            "b17bce6c44e0f680aa0f0a28a4e3865b43cdd18644a383e7a9d2f17310e5b6aa",
            1306832,
            Variant::R,
            "427c932fc143f299f6d6d1250a888230",
            "984440e0b9f77f1159f09b13d2d455292d5a9b4095037f4e8ca2a0ed982bee8f",
        );
        test(
            "7e2c813d10f06d4b8af85389bc82eb18",
            "74fc41829b88f55e62aec4749685b323",
            "7a00c480b31d851359d78fad279dcd343bcd6a5f902ac0b55da656d735dbf329",
            130160,
            Variant::V2,
            "7e2c813d10f06d4b8af85389bc82eb18",
            "6ccb68ee6fc38a6e91f546f62b8e1a64b5223a4a0ef916e6062188c4ee15a879",
        );
    }

    #[test]
    fn test_cn_slow_hash() {
        let input = hex!("5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374");
        const EXPECTED: &str = "f759588ad57e758467295443a9bd71490abff8e9dad1b95b6bf2f5d0d78387bc";
        let hash = cn_slow_hash(&input, Variant::R, 1806260);
        assert_eq!(hex::encode(hash), EXPECTED);
    }

    #[test]
    fn test_skein_hash() {
        let input: [u8; 32] =
            hex!("f759588ad57e758467295443a9bd71490abff8e9dad1b95b6bf2f5d0d78387bc");
        let mut hasher = skein::Skein512::<U32>::default();
        hasher.update(&input);
        let hash = hasher.finalize();
        let expected_hex = "64fa505e60b4c6592a4574922a19098e4c99c529d5bb99455f70b7b9a8eb0502";
        assert_eq!(hex::encode(hash), expected_hex);
    }

    #[test]
    fn test_jh() {
        let input: [u8; 32] =
            hex!("f759588ad57e758467295443a9bd71490abff8e9dad1b95b6bf2f5d0d78387bc");

        let mut hasher = jh::Jh256::default();
        hasher.update(&input);
        let hash = hasher.finalize();
        let expected_hex = "85a348a76c019e45812e1df5edc4554fe3d04b54f7955765b60ca9b5bace2813";
        assert_eq!(hex::encode(hash), expected_hex);
    }

    #[test]
    fn test_blake() {
        let input: [u8; 32] =
            hex!("f759588ad57e758467295443a9bd71490abff8e9dad1b95b6bf2f5d0d78387bc");
        let mut output = [0u8; 32];
        blake::hash(256, &input, &mut output).expect("blake hash failed");
        const EXPECTED: &str = "0ddf9a49c3695ba94e0860742a2b913ee3646e55b95918e782cefeca6e240063";
        assert_eq!(hex::encode(output), EXPECTED);
    }

    #[test]
    fn test_keccakf() {
        let mut st: [u64; 25] = [0; 25];

        keccak::keccak_p(&mut st, 24);

        let expected: [u64; 25] = [
            0xf1258f7940e1dde7,
            0x84d5ccf933c0478a,
            0xd598261ea65aa9ee,
            0xbd1547306f80494d,
            0x8b284e056253d057,
            0xff97a42d7f8e6fd4,
            0x90fee5a0a44647c4,
            0x8c5bda0cd6192e76,
            0xad30a6f71b19059c,
            0x30935ab7d08ffc64,
            0xeb5aa93f2317d635,
            0xa9a6e6260d712103,
            0x81a57c16dbcf555f,
            0x43b831cd0347c826,
            0x1f22f1a11a5569f,
            0x5e5635a21d9ae61,
            0x64befef28cc970f2,
            0x613670957bc46611,
            0xb87c5a554fd00ecb,
            0x8c3ee88a1ccf32c8,
            0x940c7922ae3a2614,
            0x1841f924a2c509e4,
            0x16f53526e70465c2,
            0x75f644e97f30a13b,
            0xeaf1ff7b5ceca249,
        ];

        assert_eq!(st, expected);
    }

    #[test]
    fn test_hash_permutations() {
        let mut state_bytes: [u8; 200] = hex!(
            "af6fe96f8cb409bdd2a61fb837e346f1a28007b0f078a8d68bc1224b6fcfcc3c39f1244db8c0af06e94173db4a54038a2f7a6a9c729928b5ec79668a30cbf5f2622fea9d7982e587e6612c4e6a1d28fdbaba4af1aea99e63322a632d514f35b4fc5cf231e9a6328efb5eb22ad2cfabe571ee8b6ef7dbc64f63185d54a771bdccd207b75e10547b4928f5dcb309192d88bf313d8bc53c8fe71da7ea93355d266c5cc8d39a1273e44b074d143849a3b302edad73c2e61f936c502f6bbabb972b616062b66d56cd8136"
        );
        const EXPECTED: &str = "31e2fb6eb8e2e376d42a53bc88166378f2a23cf9be54645ff69e8ade3aa4b7ad35040d0e3ad0ee0d8562d53a51acdf14f44de5c097c48a29f63676346194b3af13c3c45af214335a14329491081068a32ea29b3a6856e0efa737dff49d3b5dbf3f7847f058bb41d36347c19d5cd5bdb354ac64a86156c8194e19b0f62d109a8112024a7734730a2bb221c137d3034204e1e57d9cec9689bc199de684f38aeed4624b84c39675a4755ce9b69fde9d36cabd12f1aef4a5b2bb6c6126900799f2109e9b6b55d7bb3ff5";
        super::hash_permutation(&mut state_bytes);
        assert_eq!(hex::encode(state_bytes), EXPECTED);
    }

    #[test]
    fn test_extra_hashes() {
        let mut input = [0u8; 200];
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
