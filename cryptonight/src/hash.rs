use hex;
use sha3::Digest;
use std::cmp::PartialEq;
use std::io::Write;
use std::u64;

use crate::hash_v4::variant4_random_math;
use crate::{cnaes, hash_v4 as v4, variant2_int_sqrt::variant2_integer_math_sqrt_fixup};

const MEMORY: usize = 1 << 21; // 2MB scratchpad
const ITER: usize = 1 << 20;
const AES_BLOCK_SIZE: usize = 16;
const AES_KEY_SIZE: usize = 32;
const INIT_SIZE_BLK: usize = 8;
const INIT_SIZE_BYTE: usize = INIT_SIZE_BLK * AES_BLOCK_SIZE;

#[derive(PartialEq, Eq, Clone, Copy)]
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

/*
#[inline]
fn v4_reg_load(dst: &mut u32, src: &[u8]) {
    debug_assert!(src.len() >= 4, "Source must have at least 4 bytes.");
    *dst = u32::from_le_bytes([src[0], src[1], src[2], src[3]]);
}
*/

fn e2i(a: &[u8], count: usize) -> usize {
    let value: usize = u64::from_le_bytes(a[..8].try_into().unwrap()) as usize;
    (value / AES_BLOCK_SIZE) & (count - 1)
}

fn mul(a: &[u8], b: &[u8], res: &mut [u8]) {
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

fn xor_blocks(a: &mut [u8], b: &[u8]) {
    assert!(a.len() >= AES_BLOCK_SIZE);
    assert!(b.len() >= AES_BLOCK_SIZE);
    for i in 0..AES_BLOCK_SIZE {
        a[i] ^= b[i];
    }
}

fn variant2_portable_shuffle_add(
    out: &mut [u8; 16],
    a_: &[u8; 16],
    base_ptr: &mut [u8],
    offset: usize,
    variant: Variant,
) {
    if variant == Variant::V2 || variant == Variant::R {
        let chunk1_offset = offset ^ 0x10;
        let chunk2_offset = offset ^ 0x20;
        let chunk3_offset = offset ^ 0x30;

        let mut chunk1 = [
            u64::from_le_bytes(
                base_ptr[chunk1_offset..chunk1_offset + 8]
                    .try_into()
                    .unwrap(),
            ),
            u64::from_le_bytes(
                base_ptr[chunk1_offset + 8..chunk1_offset + 16]
                    .try_into()
                    .unwrap(),
            ),
        ];
        let chunk2 = [
            u64::from_le_bytes(
                base_ptr[chunk2_offset..chunk2_offset + 8]
                    .try_into()
                    .unwrap(),
            ),
            u64::from_le_bytes(
                base_ptr[chunk2_offset + 8..chunk2_offset + 16]
                    .try_into()
                    .unwrap(),
            ),
        ];
        let chunk3 = [
            u64::from_le_bytes(
                base_ptr[chunk3_offset..chunk3_offset + 8]
                    .try_into()
                    .unwrap(),
            ),
            u64::from_le_bytes(
                base_ptr[chunk3_offset + 8..chunk3_offset + 16]
                    .try_into()
                    .unwrap(),
            ),
        ];

        let b1 = [
            u64::from_le_bytes(base_ptr[16..24].try_into().unwrap()),
            u64::from_le_bytes(base_ptr[24..32].try_into().unwrap()),
        ];
        chunk1[0] = chunk3[0].wrapping_add(b1[0]).to_le();
        chunk1[1] = chunk3[1].wrapping_add(b1[1]).to_le();

        let a0 = [
            u64::from_le_bytes(a_[..8].try_into().unwrap()),
            u64::from_le_bytes(a_[8..16].try_into().unwrap()),
        ];
        base_ptr[chunk3_offset..chunk3_offset + 8].copy_from_slice(&a0[0].to_le_bytes());
        base_ptr[chunk3_offset + 8..chunk3_offset + 16].copy_from_slice(&a0[1].to_le_bytes());

        let b0 = [
            u64::from_le_bytes(base_ptr[..8].try_into().unwrap()),
            u64::from_le_bytes(base_ptr[8..16].try_into().unwrap()),
        ];
        base_ptr[chunk2_offset..chunk2_offset + 8].copy_from_slice(&chunk1[0].to_le_bytes());
        base_ptr[chunk2_offset + 8..chunk2_offset + 16].copy_from_slice(&chunk1[1].to_le_bytes());

        if variant == Variant::R {
            let mut out_copy = [
                u64::from_le_bytes(out[..8].try_into().unwrap()),
                u64::from_le_bytes(out[8..16].try_into().unwrap()),
            ];
            chunk1[0] ^= chunk2[0];
            chunk1[1] ^= chunk2[1];
            out_copy[0] ^= chunk3[0];
            out_copy[1] ^= chunk3[1];
            out_copy[0] ^= chunk1[0];
            out_copy[1] ^= chunk1[1];
            out[..8].copy_from_slice(&out_copy[0].to_le_bytes());
            out[8..16].copy_from_slice(&out_copy[1].to_le_bytes());
        }
    }
}

fn variant2_portable_integer_math(
    b: &mut [u8],
    ptr: &[u8],
    division_result: &mut u64,
    sqrt_result: &mut u64,
) {
    let tmpx = *division_result ^ (*sqrt_result << 32);
    let b0 = u64::from_le_bytes(b[0..8].try_into().unwrap()) ^ tmpx.to_le();
    b[0..8].copy_from_slice(&b0.to_le_bytes());

    let dividend = u64::from_le_bytes(ptr[8..16].try_into().unwrap());
    let divisor = (u64::from_le_bytes(ptr[0..8].try_into().unwrap())
        .wrapping_add(*sqrt_result << 1))
        | 0x80000001;
    *division_result = (dividend / divisor) as u64 + ((dividend % divisor) << 32);

    let sqrt_input =
        u64::from_le_bytes(ptr[0..8].try_into().unwrap()).wrapping_add(*division_result);

    *sqrt_result =
        ((sqrt_input as f64 + 18446744073709551616.0).sqrt() * 2.0 - 8589934592.0) as u64;
    variant2_integer_math_sqrt_fixup(sqrt_result, sqrt_input);
}

fn variant2_integer_math(
    c1: &mut [u8; 16],
    c2: &[u8; 16],
    division_result: &mut u64,
    sqrt_result: &mut u64,
    variant: i32,
) {
    const U32_MASK: u64 = u32::MAX as u64;

    if variant == 2 || variant == 3 {
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
        *sqrt_result =
            ((sqrt_input as f64 + 18446744073709551616.0).sqrt() * 2.0 - 8589934592.0) as u64;

        let sqrt_div2 = *sqrt_result >> 1;
        let lsb = *sqrt_result & 1;
        let r2 = sqrt_div2
            .wrapping_mul(sqrt_div2 + lsb)
            .wrapping_add(*sqrt_result << 32);

        // TODO: Find way to get code coverage here.
        if r2.wrapping_add(lsb) > sqrt_input {
            println!("made it to fixup 1");
            *sqrt_result = sqrt_result.wrapping_sub(u64::MAX);
        }
        if r2.wrapping_add(1 << 32) < sqrt_input.wrapping_sub(sqrt_div2) {
            println!("made it to fixup 2");
            *sqrt_result = sqrt_result.wrapping_add(1);
        }
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

fn cn_slow_hash(data: &[u8], variant: Variant, height: u64) -> ([u8; 16], [u8; 32]) {
    // let long_state: [u8; MEMORY];
    // let text: [u8; INIT_SIZE_BYTE];
    let mut a = [0u8; AES_BLOCK_SIZE];
    let mut b = [0u8; AES_BLOCK_SIZE * 2];
    let mut aes_key = [0u8; AES_KEY_SIZE];

    let mut state = CnSlowHashState::default();
    keccak1600(data, &mut state.get_keccak_bytes_mut());
    //println!("keccak_state_bytes: {}", hex::encode(state.get_keccak_bytes()));
    aes_key.copy_from_slice(&state.get_k()[0..AES_KEY_SIZE]);
    let aes_expanded_key = cnaes::key_extend(&aes_key);
    let mut text: [u8; INIT_SIZE_BYTE] = state.get_init().try_into().unwrap();
    //println!("text(1): {}", hex::encode(&text));
    //println!("keccak_state_bytes: {}", hex::encode(state.get_keccak_bytes()));

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

    let mut c1 = [0u8; AES_BLOCK_SIZE];
    let mut c2 = [0u8; AES_BLOCK_SIZE];
    let mut a1 = [0u8; AES_BLOCK_SIZE];
    let mut d = [0u8; AES_BLOCK_SIZE];

    for i in 0..ITER / 2 {
        /* Dependency chain: address -> read value ------+
         * written value <-+ hard function (AES or MUL) <+
         * next address  <-+
         */
        // Iteration
        let mut j = e2i(&a[..8], MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE;
        copy_block(&mut c1[..], &long_state[j..j + AES_BLOCK_SIZE]);
        cnaes::aesb_single_round(&mut c1, &a);
        variant2_portable_shuffle_add(&mut c1, &a, &mut long_state, j, variant);
        if i == 0 {
            println!("i={}, j={}", i, j);
            println!("c1: {}", hex::encode(&c1));
            let mut output = [0u8; 32];
            blake::hash(256, &long_state, &mut output).expect("blake hash failed");
            println!("long_state: {}", hex::encode(&output));
        }
        copy_block(
            &mut long_state[j..j + AES_BLOCK_SIZE],
            &c1[..AES_BLOCK_SIZE],
        );
        xor_blocks(&mut long_state[j..j + AES_BLOCK_SIZE], &b[..AES_BLOCK_SIZE]);
        assert_eq!(j, e2i(&a[..8], MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE);
        variant1_1(&mut long_state[j..], variant);

        /* Iteration 2 */
        j = e2i(&mut c1, MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE;
        copy_block(&mut c2, &long_state[j..]);
        copy_block(&mut a1, &a);
        variant2_portable_integer_math(&mut c2, &c1, &mut division_result, &mut sqrt_result);
        variant4_random_math(
            &mut a1,
            &mut c2,
            &mut r[0..9].try_into().unwrap(),
            &mut b[0..4].try_into().unwrap(),
            &mut b[AES_BLOCK_SIZE..AES_BLOCK_SIZE + 8].try_into().unwrap(),
            &code,
        );
        mul(&c1, &c2, &mut d);

        // VARIANT2_2_PORTABLE
        if variant == Variant::V2 || variant == Variant::R {
            xor_blocks(&mut long_state[j ^ 0x10..], &d);
            xor_blocks(&mut d, &long_state[j ^ 0x20..]);
        }

        variant2_portable_shuffle_add(&mut c1, &a, &mut long_state, j, variant);
        sum_half_blocks(&mut a1, &d);
        swap_blocks(&mut a1, &mut c2);
        xor_blocks(&mut a1, &mut c2);
        // VARIANT1_2
        if variant == Variant::V1 {
            xor64(&mut c2[8..16], &tweak1_2);
        }
        copy_block(&mut long_state[j..], &c2);
        if variant == Variant::V2 || variant == Variant::R {
            let (mut b_start, b_rest) = b.split_at_mut(AES_BLOCK_SIZE);
            copy_block(&mut b_start, &b_rest);
        }
        copy_block(&mut b, &c1);
        copy_block(&mut a, &a1);
    }

    (a, b)
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
    fn test_mul() {
        let test = |a_hex: &str, b_hex: &str, expected_hex: &str| {
            let a = hex::decode(a_hex).unwrap();
            let b = hex::decode(b_hex).unwrap();
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
    fn test_variant2_integer_math() {
        let test = |c1_hex: &str,
                    c2_hex: &str,
                    division_result: u64,
                    sqrt_result: u64,
                    c1_hex_end: &str,
                    division_result_end: u64,
                    sqrt_result_end: u64| {
            let c1_vec = hex::decode(c1_hex).unwrap();
            let c2_vec = hex::decode(c2_hex).unwrap();
            let mut c1: [u8; 16] = c1_vec.as_slice().try_into().unwrap();
            let c2: [u8; 16] = c2_vec.as_slice().try_into().unwrap();
            let mut division_result = division_result;
            let mut sqrt_result = sqrt_result;
            super::variant2_integer_math(&mut c1, &c2, &mut division_result, &mut sqrt_result, 2);
            assert_eq!(hex::encode(c1), c1_hex_end, "c1 does not match");
            assert_eq!(
                division_result, division_result_end,
                "division_result does not match"
            );
            assert_eq!(sqrt_result, sqrt_result_end, "sqrt_result does not match");
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
    fn test_cn_slow_hash() {
        let input = hex::decode("5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374").unwrap();
        let (a, b) = cn_slow_hash(&input, Variant::R, 1806260);

        let a_hex = "9f100d9490b133036a6876b3079b1797";
        let b_hex = "9f100d9490b133036a6876b3079b17979780ff47e3d1f45e162faef2e8d3df48";

        assert_eq!(hex::encode(a), a_hex);
        assert_eq!(hex::encode(b), b_hex);
    }
}
