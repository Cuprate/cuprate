use std::{cmp::PartialEq, io::Write, mem::swap};

use cnaes::{AES_BLOCK_SIZE, CN_AES_KEY_SIZE};
use digest::Digest as _;
use groestl::Groestl256;
use jh::Jh256;
use skein::{consts::U32, Skein512};

use crate::{
    blake256::{Blake256, Digest as _},
    cnaes, hash_v2 as v2, hash_v4 as v4,
    util::{subarray, subarray_copy, subarray_mut},
};

pub(crate) const MEMORY: usize = 1 << 21; // 2MB scratchpad
pub(crate) const MEMORY_BLOCKS: usize = MEMORY / AES_BLOCK_SIZE;

const ITER: usize = 1 << 20;
const AES_KEY_SIZE: usize = CN_AES_KEY_SIZE;
const INIT_BLOCKS: usize = 8;
const INIT_SIZE_BYTE: usize = INIT_BLOCKS * AES_BLOCK_SIZE;

const KECCAK1600_BYTE_SIZE: usize = 200;

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum Variant {
    V0,
    V1,
    V2,
    R,
}

/// Equivalent struct in the C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L469-L477>
struct CnSlowHashState {
    b: [u8; KECCAK1600_BYTE_SIZE],
}

impl Default for CnSlowHashState {
    fn default() -> Self {
        Self {
            b: [0; KECCAK1600_BYTE_SIZE],
        }
    }
}

impl CnSlowHashState {
    const fn get_keccak_bytes(&self) -> &[u8; KECCAK1600_BYTE_SIZE] {
        &self.b
    }

    const fn get_keccak_bytes_mut(&mut self) -> &mut [u8; KECCAK1600_BYTE_SIZE] {
        &mut self.b
    }

    fn get_keccak_word(&self, index: usize) -> u64 {
        u64::from_le_bytes(subarray_copy(&self.b, index * 8))
    }

    fn get_k(&self) -> [u128; 4] {
        [
            u128::from_le_bytes(subarray_copy(&self.b, 0)),
            u128::from_le_bytes(subarray_copy(&self.b, 16)),
            u128::from_le_bytes(subarray_copy(&self.b, 32)),
            u128::from_le_bytes(subarray_copy(&self.b, 48)),
        ]
    }

    fn get_aes_key0(&self) -> &[u8; AES_KEY_SIZE] {
        subarray(&self.b, 0)
    }

    fn get_aes_key1(&self) -> &[u8; AES_KEY_SIZE] {
        subarray(&self.b, AES_KEY_SIZE)
    }

    #[inline]
    fn get_init(&self) -> [u128; INIT_BLOCKS] {
        let mut init = [0_u128; INIT_BLOCKS];
        for (i, block) in init.iter_mut().enumerate() {
            *block = u128::from_le_bytes(subarray_copy(&self.b, 64 + i * AES_BLOCK_SIZE));
        }
        init
    }

    fn set_init(&mut self, init: &[u128; INIT_BLOCKS]) {
        for (i, block) in init.iter().enumerate() {
            self.b[64 + i * AES_BLOCK_SIZE..64 + (i + 1) * AES_BLOCK_SIZE]
                .copy_from_slice(&block.to_le_bytes());
        }
    }
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/hash.c#L38-L47>
fn hash_permutation(b: &mut [u8; KECCAK1600_BYTE_SIZE]) {
    let mut state = [0_u64; 25];

    for (i, state_i) in state.iter_mut().enumerate() {
        *state_i = u64::from_le_bytes(subarray_copy(b, i * 8));
    }

    // Same as keccakf in the C code
    keccak::keccak_p(&mut state, 24);

    for (i, chunk) in state.iter().enumerate() {
        b[i * 8..i * 8 + 8].copy_from_slice(&chunk.to_le_bytes());
    }
}

fn keccak1600(input: &[u8], out: &mut [u8; KECCAK1600_BYTE_SIZE]) {
    let mut hasher = sha3::Keccak256Full::new();
    _ = hasher.write(input).unwrap();
    let result = hasher.finalize();
    out.copy_from_slice(result.as_ref());
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L1709C1-L1709C27>
#[inline]
#[expect(clippy::cast_possible_truncation)]
const fn e2i(a: u128) -> usize {
    const MASK: u64 = ((MEMORY_BLOCKS) - 1) as u64;

    // truncates upper 64 bits before dividing
    let value = (a as u64) / (AES_BLOCK_SIZE as u64);

    // mask is 0x1ffff, so no data is truncated if usize is 32 bits
    (value & MASK) as usize
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L1711-L1720>
#[expect(clippy::cast_possible_truncation)]
fn mul(a: u64, b: u64) -> u128 {
    let product = u128::from(a).wrapping_mul(u128::from(b));
    let hi = (product >> 64) as u64;
    let lo = product as u64;

    // swap hi and low, so this isn't just a multiply
    (u128::from(lo) << 64) | u128::from(hi)
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L1722-L1733>
#[expect(clippy::cast_possible_truncation)]
fn sum_half_blocks(a: u128, b: u128) -> u128 {
    let a_low = a as u64;
    let b_low = b as u64;
    let sum_low = a_low.wrapping_add(b_low);

    let a_high = (a >> 64) as u64;
    let b_high = (b >> 64) as u64;
    let sum_high = a_high.wrapping_add(b_high);

    (u128::from(sum_high) << 64) | u128::from(sum_low)
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L144-L151>
fn variant1_init(state: &CnSlowHashState, data: &[u8], variant: Variant) -> u64 {
    const NONCE_PTR_INDEX: usize = 35;

    if variant != Variant::V1 {
        return 0;
    }

    assert!(
        data.len() >= 43,
        "Cryptonight variant 1 needs at least 43 bytes of data"
    );

    let mut tweak1_2 = u64::from_le_bytes(subarray_copy(&state.get_keccak_bytes(), 192));
    tweak1_2 ^= u64::from_le_bytes(subarray_copy(data, NONCE_PTR_INDEX));

    tweak1_2
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L120-L127>
fn variant1_1(p: &mut u128, variant: Variant) {
    const MASK_BYTE11: u128 = !(0xFF << (11 * 8)); // all bits except the 11th byte are ones
    const TABLE: u32 = 0x75310_u32;

    #[expect(clippy::cast_possible_truncation)]
    if variant == Variant::V1 {
        let old_byte11 = (*p >> (11 * 8)) as u8;
        let index = (((old_byte11 >> 3) & 6) | (old_byte11 & 1)) << 1;
        let new_byte11 = old_byte11 ^ ((TABLE >> index) & 0x30) as u8;
        *p = (*p & MASK_BYTE11) | (u128::from(new_byte11) << (11 * 8));
    }
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L129C1-L133C13>
fn variant1_2(c2: &mut u128, tweak1_2: u64, variant: Variant) {
    if variant == Variant::V1 {
        *c2 ^= u128::from(tweak1_2) << 64;
    }
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L171-L181>
fn variant_2_init(b: &mut [u128; 2], state: &CnSlowHashState, variant: Variant) -> (u64, u64) {
    if variant != Variant::V2 && variant != Variant::R {
        return (0, 0);
    }

    let keccak_state_bytes = state.get_keccak_bytes();
    b[1] = u128::from_le_bytes(subarray_copy(keccak_state_bytes, 64))
        ^ u128::from_le_bytes(subarray_copy(keccak_state_bytes, 80));
    let division_result = state.get_keccak_word(12);
    let sqrt_result = state.get_keccak_word(13);

    (division_result, sqrt_result)
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L295-L299>
fn variant_2_2(long_state: &mut [u128; MEMORY_BLOCKS], j: usize, d: &mut u128, variant: Variant) {
    if variant == Variant::V2 {
        let chunk1_start = j ^ 0x1;
        let chunk2_start = j ^ 0x2;
        long_state[chunk1_start] ^= *d;
        *d ^= long_state[chunk2_start];
    }
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L319-L334>
/// The compiler would inline this code even without the `#[inline]` attribute, but we'd like
/// to avoid coping `r` and `code` between stack addresses.
#[inline]
fn variant4_math_init(
    height: u64,
    state: &CnSlowHashState,
    variant: Variant,
) -> (
    [u32; v4::NUM_INSTRUCTIONS_MAX + 1],
    [v4::Instruction; v4::NUM_INSTRUCTIONS_MAX + 1],
) {
    let mut r = [0_u32; v4::NUM_INSTRUCTIONS_MAX + 1];
    let mut code = [v4::Instruction::default(); v4::NUM_INSTRUCTIONS_MAX + 1];
    let keccak_state_bytes = state.get_keccak_bytes();
    if variant == Variant::R {
        for (i, r_i) in r.iter_mut().enumerate().take(4) {
            *r_i = u32::from_le_bytes(subarray_copy(keccak_state_bytes, (24 + i) * 4));
        }
        v4::random_math_init(&mut code, height);
    }
    (r, code)
}

fn extra_hashes(input: &[u8; KECCAK1600_BYTE_SIZE]) -> [u8; 32] {
    match input[0] & 0x3 {
        0 => Blake256::digest(input),
        1 => Groestl256::digest(input).into(),
        2 => Jh256::digest(input).into(),
        3 => Skein512::<U32>::digest(input).into(),
        _ => unreachable!(),
    }
}

/// Original C code:
/// <https://github.com/monero-project/monero/blob/v0.18.3.4/src/crypto/slow-hash.c#L1776-L1873>
#[expect(clippy::cast_possible_truncation)]
pub(crate) fn cn_slow_hash(data: &[u8], variant: Variant, height: u64) -> [u8; 32] {
    let mut state = CnSlowHashState::default();
    keccak1600(data, state.get_keccak_bytes_mut());
    let aes_expanded_key = cnaes::key_extend(state.get_aes_key0());
    let mut text = state.get_init();

    let tweak1_2 = variant1_init(&state, data, variant);
    let mut b = [0_u128; 2];
    let (mut division_result, mut sqrt_result) = variant_2_init(&mut b, &state, variant);
    let (mut r, code) = variant4_math_init(height, &state, variant);

    // Use a vector so the memory is allocated on the heap. We might have 2MB
    // available on the stack, but that optimization would only be meaningful if
    // this code was still used for mining.
    let mut long_state: Vec<u128> = Vec::with_capacity(MEMORY_BLOCKS);

    for i in 0..MEMORY_BLOCKS {
        let block = &mut text[i % INIT_BLOCKS];
        *block = cnaes::aesb_pseudo_round(*block, &aes_expanded_key);
        long_state.push(*block);
    }

    // Treat long_state as an array now that it's initialized on the heap
    let long_state: &mut [u128; MEMORY_BLOCKS] = subarray_mut(&mut long_state, 0);

    let k = state.get_k();
    let mut a = k[0] ^ k[2];
    b[0] = k[1] ^ k[3];

    let mut c1;
    let mut c2;
    let mut a1;

    for _ in 0..ITER / 2 {
        /* Dependency chain: address -> read value ------+
         * written value <-+ hard function (AES or MUL) <+
         * next address  <-+
         */
        // Iteration
        let mut j = e2i(a);
        c1 = long_state[j];
        cnaes::aesb_single_round(&mut c1, a);
        v2::variant2_shuffle_add(&mut c1, a, &b, long_state, j, variant);

        long_state[j] = c1 ^ b[0];
        variant1_1(&mut long_state[j], variant);

        /* Iteration 2 */
        j = e2i(c1);
        c2 = long_state[j];

        a1 = a;
        v2::variant2_integer_math(&mut c2, c1, &mut division_result, &mut sqrt_result, variant);
        v4::variant4_random_math(&mut a1, &mut c2, subarray_mut(&mut r, 0), &b, &code);
        let mut d = mul(c1 as u64, c2 as u64);
        variant_2_2(long_state, j, &mut d, variant);
        v2::variant2_shuffle_add(&mut c1, a, &b, long_state, j, variant);
        a1 = sum_half_blocks(a1, d);
        swap(&mut a1, &mut c2);
        a1 ^= c2;
        variant1_2(&mut c2, tweak1_2, variant);
        long_state[j] = c2;

        if variant == Variant::V2 || variant == Variant::R {
            b[1] = b[0];
        }
        b[0] = c1;
        a = a1;
    }

    let mut text = state.get_init();
    let aes_expanded_key = cnaes::key_extend(state.get_aes_key1());
    for i in 0..MEMORY / INIT_SIZE_BYTE {
        for (j, block) in text.iter_mut().enumerate() {
            let ls_index = i * INIT_BLOCKS + j;
            *block ^= long_state[ls_index];
            *block = cnaes::aesb_pseudo_round(*block, &aes_expanded_key);
        }
    }
    state.set_init(&text);

    hash_permutation(state.get_keccak_bytes_mut());

    extra_hashes(state.get_keccak_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::hex_to_array;

    #[test]
    fn test_keccak1600() {
        let input: [u8; 44] = hex_to_array(
            "5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374"
        );
        let mut output = [0_u8; KECCAK1600_BYTE_SIZE];
        keccak1600(&input, &mut output);
        let output_hex = "af6fe96f8cb409bdd2a61fb837e346f1a28007b0f078a8d68bc1224b6fcfcc3c39f1244db8c0af06e94173db4a54038a2f7a6a9c729928b5ec79668a30cbf5f266110665e23e891ea4ee2337fb304b35bf8d9c2e4c3524e52e62db67b0b170487a68a34f8026a81b35dc835c60b356d2c411ad227b6c67e30e9b57ba34b3cf27fccecae972850cf3889bb3ff8347b55a5710d58086973d12d75a3340a39430b65ee2f4be27c21e7b39f47341dd036fe13bf43bb2c55bce498a3adcbf07397ea66062b66d56cd8136";
        assert_eq!(hex::encode(output), output_hex);
    }

    #[test]
    fn test_mul() {
        let test = |a_hex: &str, b_hex: &str, expected_hex: &str| {
            let a = u64::from_le_bytes(hex_to_array(a_hex));
            let b = u64::from_le_bytes(hex_to_array(b_hex));
            let res = mul(a, b);
            assert_eq!(hex::encode(res.to_le_bytes()), expected_hex);
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
    fn test_hash_permutations() {
        let mut state_bytes: [u8; KECCAK1600_BYTE_SIZE] = hex_to_array(
            "af6fe96f8cb409bdd2a61fb837e346f1a28007b0f078a8d68bc1224b6fcfcc3c39f1244db8c0af06e94173db4a54038a2f7a6a9c729928b5ec79668a30cbf5f2622fea9d7982e587e6612c4e6a1d28fdbaba4af1aea99e63322a632d514f35b4fc5cf231e9a6328efb5eb22ad2cfabe571ee8b6ef7dbc64f63185d54a771bdccd207b75e10547b4928f5dcb309192d88bf313d8bc53c8fe71da7ea93355d266c5cc8d39a1273e44b074d143849a3b302edad73c2e61f936c502f6bbabb972b616062b66d56cd8136"
        );
        const EXPECTED: &str = "31e2fb6eb8e2e376d42a53bc88166378f2a23cf9be54645ff69e8ade3aa4b7ad35040d0e3ad0ee0d8562d53a51acdf14f44de5c097c48a29f63676346194b3af13c3c45af214335a14329491081068a32ea29b3a6856e0efa737dff49d3b5dbf3f7847f058bb41d36347c19d5cd5bdb354ac64a86156c8194e19b0f62d109a8112024a7734730a2bb221c137d3034204e1e57d9cec9689bc199de684f38aeed4624b84c39675a4755ce9b69fde9d36cabd12f1aef4a5b2bb6c6126900799f2109e9b6b55d7bb3ff5";
        hash_permutation(&mut state_bytes);
        assert_eq!(hex::encode(state_bytes), EXPECTED);
    }

    #[test]
    fn test_extra_hashes() {
        let mut input = [0_u8; KECCAK1600_BYTE_SIZE];
        for (i, val) in input.iter_mut().enumerate() {
            *val = u8::try_from(i & 0xFF).unwrap();
        }

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
            input[0] = u8::try_from(i).unwrap();
            let output = extra_hashes(&input);
            assert_eq!(hex::encode(output), *expected, "hash {i}");
        }
    }

    #[test]
    fn test_cn_slow_hash() {
        let test = |input_hex: &str,
                    expected_v0_hex: &str,
                    expected_v1_hex: &str,
                    expected_v2_hex: &str,
                    expected_vr_hex: &str,
                    vr_height: u64| {
            let input = hex::decode(input_hex).unwrap();
            assert_eq!(
                hex::encode(cn_slow_hash(&input, Variant::V0, 0)),
                expected_v0_hex
            );
            assert_eq!(
                hex::encode(cn_slow_hash(&input, Variant::V1, 0)),
                expected_v1_hex
            );
            assert_eq!(
                hex::encode(cn_slow_hash(&input, Variant::V2, 0)),
                expected_v2_hex
            );
            assert_eq!(
                hex::encode(cn_slow_hash(&input, Variant::R, vr_height)),
                expected_vr_hex
            );
        };
        test(
            "a83cd815319596c6e4fbf2ff9399ce99eb092f58b75c351a7be64a65a118cee031c06a8542b758a15b8a7e",
            "19dff098bf4f330c466480c6bf59cf89c5b4a9692a8941e7625435275e7b12e5",
            "89def5411f4c877e75a0ae9a49c73e63071acedf69e730d9d56425c01ecf0c9a",
            "4c058a38985b6b064d85562f0bd2ebfbaa2915de6ec90307b660b5525ef7d7c3",
            "4729aa8a82fcf9b014baf5c2d2fdeadc3ea58bcca7934bb98359caea8f0227e8",
            1901601,
        );
        test(
            "0020a84a1806a9cfd957bfd243de587891d6e732322289a9de3db5b15aceacbf1d5b94d04f143bd652bc2d5c6980c2db58007d03b4e5cf94829b06b13ccd61e9edb2ea910876e2eaff62304658a07144236ac3492edfbdee9077f62622d6b9b56173b0eb822391857a33c17e4fed4a54f42f2ea7d85e9246e6879a98c9576240477060ef4f4c88056d08acbd36825b314cfba95082aa66ab6bc89b9831ece3abcc8e7764eb",
            "24b03eefa6fa730cfceb1fe3043dacdff9242f69a3aff86b9d75e08d5ce6903a",
            "b624fcd7345589b2531f29a9d47ab53f4f90caaf3e76f20aa3573d73421e55a2",
            "6426f2a9e3274926b41965f389ec40625f32d1ae6decfff8966d5af208789bf1",
            "efb1c48e81b58e0f1b0d574ab142dedbf3b089a8fc6c9b50f2bef507147d8e02",
            2066323,
        );
        test(
            "93935e3d10b73a45f77dfbbbc08c819ca65a2d279354f945d5c2b244cea6af82d9b39fd293b7ec136e2a0e584ec6faf584d669226a976f90dc61acc62f13de42a04d68dc2e80310a692fe2e1a88f05fe0e370117dd9b8edaa9bacc827571c8ba8b55dece38bd67fc22080d10b94dae5fb7caf68297b240fe026e19140c88873ee764a8a8ffcb3a1d76fcbd530dfd1b7e6cccda31362ec9b2b6eb420aba7c6088685ae5d75cdd1334dbd979b916a0f1426563e8e6ad778f1110d49ef34e446346dbc9c358b29e6f923351a8368daacf6661c19c0c1658c8f7d0386ee9f61e729e5f6fdcb4d740655afb6d2191801c0126fd1f18b82604dc753dce507313c4ade7e6c01d470f18b701e21f",
            "e6bb2a8e6331b816749c0dde5036a417c0f33e373177e4e93b7bc2452d6c3fe4",
            "e162d6dc79a6f2cc4497dcc65b6f7395fb9a8df4eec53e87f49c0d14bd36318a",
            "8ecf8c005bd5539c0c16cac9f2e5744f641de5b886f069f31e2d90ac8287479b",
            "6a26f39a459325b43aaaa38d2f746678907cb4210c1335096f8300291ab22797",
            2045824,
        );
        test(
            "4c1818705e30a631e152fd7f21f3fb836a0e441d7450a7fd38f119f95c650a66ecb79fb6d60db04dce28e0798c8e5cdb9f13738da06484b2ab302b548ca0736d0a58ace27be67b07219e9db7e50c4243e9e2a6b60eccac1b9b4b266f85d3814a846239bccfead53b752ead9ddacc9abf100dd9889c3fd7b064f4102f864df79138305b61937f5bb458cfdda2e3e1dc553e06f90efde088fb995314c539295899e31509c4198d1ad056b5fe9ac7d7e3959a3d73c7f8b5203f9f21c688b196841abf8511630864f304ba44d6fbf627164596c6cca56abbb8a3a005abe6a493ac08a3847f65cb3f90ef531d233b9953b86d7a659ec82cfb129f98ac5852fa73a2b84920669ec8d14fe1ba20f9ad488b86d7d344c143474198cdfbd6cf67df3e3dfb293c62361c3e3a4246fcf1558cdf69ec10f63770f9a8a23ff80f3ca46ee5ca35ba384bc3f8eb0d159e58b649045e87b0095e800673e0f33d4ce2cfdde727009fa0c74d9866bcb43477a3075149068868f58a33f7880d609ad2",
            "3f3e491848ba252d497231c696e1538a6e1a7f6c79a8a1b2764c5810f9c4d38c",
            "9e272f82a466c86a1d093a07dcf7c471d8e31a056dfa45f902bdc4ebeae9bf3f",
            "8a5260d0b5a4a0e82827daff7072f95822e316f2bdbb5f021cb84a9d44f76127",
            "94d96aeca25bb37bde8497e9e45f5a695ee6e7a6c5ba489efe0bb0a10494eba4",
            2099507,
        );
        test(
            "b27c89ca34c63efe96732f8cda3eceeeb12304e0e6262e9463d516994adafb11ba2db10f9de8e5fb3b271f2a10065a77a7da998c07f679d7b36bb6ed0051e6f5f6871247fa167dc2d0b19f55f9ca283e6c4486967ac7783ed4f6c292e536aaf8425e436bcc38e55f942f797b2fdc9d4d73cbe4737d2ca849b783c4560ea97cfbb448d444f46bd360aad1c3077e032fe0c43f95e4d5e263b8fb8c2eb55aa6d6403c1075883b4fe657e7b79341b3f88ad48817caaa6cadaf0483c7d643a9b2f0eae8c3470f1df4ffae2453f179c90c0efd51fe8fbfe85d9e4d306266eab9fc5739e3869767aac8ef91e4df881e2595dc235e1ea01579b21fd964f6e66269907bb19f2133534bb34be607dd582775475f81263ed5cf09a9c11a0c290de2dc9fa1a99247fc3c6031cbe46dd833dbaa7722f7dcbb5fa037c7bdc6f4b58e54e3b8157aa893eb8038eb6a0d4364c5fc66352065883e8da1b06840c8de1990c00fb88373b9971351cae3762bef7bcce71e269f76b832321e4b83528e8379132f43fd562553d3745e44dee5a0ffa8a6245c8c228831f22b865feeb7499c54b841c69279a822491eb76600afb25b22885e51822e4ef48da78756dfea36297f2ba999fe99bb80f1432c5e858d52a6ed6c",
            "83698a19ea0c50b8b411e3f7206522ef96fd0419cc9fed72de6ecb43b4e8b909",
            "80b760cf63d938d73dd2d13670f9425dc3a50dcb66e237facdf75ea3e6e18103",
            "6a5a069cdbb29914b1e189c5665e7a7ce3bb647bc7239dc855950e5c0906fe14",
            "e05b7377f8099a0c0b1dc53121fc4070d65cf3b896a06bea6a6702a37201e79a",
            1930040,
        );
        test(
            "5641ce08706166c97f2b13052aed63ed360565f0c74db968cd2cac2ed415cb173ab3583ab771ef3d126648afad09da2b35038f124238c8ca916c3e3fa4c57626965397d1bd865b904dcc3a6398eb2b501d6fd82bb28d80b38da6d4f333b260a60375faea13e3fd9ac2d2745e660965ee2624d68114fce6520f62796dcc4160d1f3cb55e48bea53911b91165782b52e05a02f71368379c428b196ef921f832383043421447784f75cc3bc5dc33ef00acdee9a77cfda31d6bf5e4bc322d09c97977aa826cf02f87a9b2d261689f25605b12997bc51cde56f6b95eb643e7ad10ce6f6b9cc1c9513bcc69fdff3ece5c7e9fe989c1f0797b8a6c0eaf17aa63c173ee7ce4a7439a93dccca735cdbaf2ac52c2626ca6dd18d190c28c7bcb74dcd5e831cfa691a9e02f9e1fc4d991d19f6c771d6a317d2335a5faef74b9485f3b708a06f9a3779d4ce1e224c1fd4c80ed682374988e9a80c16c489864cbfb9619eac25e7a529f75963dcfabcaae75921ff4f37e90771683c365dbc6e49394ed42a4930308f61dcc5493f6fbc61f089d31e38304f88b03090db752cea1ed149b01347277563c372fb9aabfa770c5324267b3a1a0c7d95ec5d787dcc6955a6075e7acf51b6f9498bbe31ca3a134fa4870e3f3d85aa8a06fdc260f67843b030b55eb57e386769481fbdc27d3a42cb7c67b626067be82008bc922d0a13cf2819426a434f867688d4c866a1323a3afd5f0d27e5f706aff5d3ae8e808dcbbbb527eba451a3fa92012f5dd450c8727d9812f4de6b35c9864b94112ea0e1a0236e905f16188be5bd909318945dbaa48c7b48537cab",
            "5e63777a37718d23d4d27df6457c3287e5df76d57e8b1576dedeaa0b48f686ba",
            "34b967683a272498d46eea6f0075ce8799e8b3871fcbb8e8af2016f841b0a563",
            "17173452105a1d8a797d46881af00f131276c2c46aedecebee357d3400948690",
            "23fb53795f04ff342508044091e9d5bc5704e637979de4491d420620f91515ed",
            2013003,
        );
        test(
            "381172bcf453dbfa7f0015823367545d83483a6c805355f26b1cb4019391c62b227530102bc19272560d2160642251ee92d009c478210e77d5fc0edd20c332648058d7a5f052423023d651eaf415bfdd6e03588afc2ccadc0b63dd187a3e585802fb6e14ee753dc778688856f9619a883de3f68300088477f26aee159ed8f67296e034e020f1577eccd56c9e5ffbf4695e756635aefa4140e346fd3977b159a950f71b506a3ffb3cb1802914ce81df8ce8ca062953eeafa7dd8fa6cdcdd85dcebdc43d411da2befff42f250c7de4a29793f1105bed7f1d7e0adcdabde04060f0370c6f66075441e06447709519cf8fa5cf89e7965399323254f0faab58c095107a61b8f07bc9a368a538c2fc2939daaaf8562c70c3d9c878821df45547caf4794e0a3e62f7f14ef4004d31e9a8feee7c9e97721d82001646e3c32551a7ac89b7cc17be74beaade5f613aa30a5b3077859c579dbdef27e27fd219947c5e5e8456c37da5ebd7fe3f1cf369ded0451098843e197db2d82788fe0ec39cb0a855cbd6a2d54aeba7f57fcea8fdfb3f153075d2b9c28dfeb3582c4f19037c45e39bfa9500fb7144f836d82c65c166f901605fb0e298396dac47d8ea44ac00ae61b16c5c53e93390698b7d6704c6a413f8244a96bcb37909404a0e1ed8219a388936e2c3a3038598aff37949b2e1f3e4b64945087914c1a24f957767e583fd875fed316aa02adce9ac1325cd88e5287c3fd7e39e8618104864297d979d27ccdb66b28005b3ef8ef2e8ebd17a1ffb5f466241a521854ef5ffc41af918bd3a4bfcef4d56fb506ece4f4e043b02815d98c64f8d3a39fd2ba5cb788cddf83e7c970113414c0b745b260336c4c376d78ccdd87c1bb1e409145c912584194cd95848039d265674b00d24a7f304d2526317bd3a60858eb05eaf9b2047905f6104371661bbeebc0e06bb6f20",
            "54ff2f094bb647ff95d3fac1411021beb2519ed4921654a464ca1d91086cf566",
            "26128757abddf96cf31e24b752c4f49b9f7e33b17719923412ef681a8b0b80d7",
            "a2553f122fb1cd293c3d3c442705cc9875705fcc374fa0b8944bae52ea818037",
            "64e11fbf279c8ef768450c5516106270f960b9c514bf945cca82a08e2db553a8",
            2098488,
        );
        test(
            "45d3cd4ada6cd0689c08ba902effa67f424b4a57da5512a9fbddf343b4872f48953485c0dd0f38a96ee61e655195354a8c7c9d73ede4581eb943fcb4bc376bae314d7853350ae805587d00958a63dc547ae4b7b217f39fff12ae684c18e1533a6acb1aba6551fc6ee78aa103543295bb81a720e3dc538d1ebf156df8510bec0960e55d853dbcdc360e3c7a39a5f4393c97b1317ea57b48d8413b67295b89860c5fe602b4a0b05724e5e1563f229645569703c3acea69d75a8ff72efb0539340747333735c74d0ad6126ee22f2b5b47b601af4f6c36f9a7227c457770dcbd31e85b63b57da98f4c6dc926afdba99851b44365f99103841c2887f4490daa63cf10954b4a8c354346307dfb3d5e2762625bdde96b6bbcf3e7af7d7bdc7a2d58ce607e97bc2e8cd39f9871debbe7b36e3652f54f54d02d5470c7662ced04435b3771ba9264c107cb0b513844ce7538d7529db455e0960dce9e309d1978a7d0d5debe55dd37cc8b1cbf1b41bdda5413ed68f51db2ce70d31232be4095dc4c4b82f9a46d8fa278970bdc72632e74eaf6ca756ca72e6f5be75823b0502ef8deefc249f8a9d37a8778ef1429cd4ad1b8c9a53e6e179d6f62f5e966a762da0db3b6974ca79c5a8be615bf57bd11a31a1b365b7f47f71a041a0c46ca18c7af20a09e22085e767948860c58e84ce56c6646d1737d4df9714362deff97ccf56021ad85ba01da06bdc46bd377cca8c731ecad9fa18013ae0a567220c4b99796d3cde96eef16aca8bf10d4e1914faeef35e505e44153eb3c2e1b0cb366751b993e662a828eb5f824ab138aa7504c8f2bf92f36a9c4c978ad064dc4e877fa37f0cbb1fd2f02eefd8b54714a7952b2efe522ae00315fd147174bb650314e20395bc13537b65aef11ebb1e54a34f169b11b895731c6eaa95ca6ea813c7aae22d19720c30265e0d7242afc4ef7fe1a685eb2d48d18b135ec6656bcb1538a6ff0086e6949be38b4d88a95279fac4475bea3c91f3cfbbff56f0896502417a8e2678e98ce4e0e184d7ba62add18082edfdecc36b79734e1d11bb62593de91d9138945caeb0d0e8e070a397388c5b010720fc9829f22fc80ae1f39b469aab343",
            "26465b05a2b7d822a180e5da853c3bb31cfb541ac57bb4082ed6baad2b7a275d",
            "3914d155ab3b103a29ca1733f165739dbba3fd70847680982c72378097644f5b",
            "47f117624851b9f56ecd40dce1a345348c7abea4223e22c4bec1b992001c05b0",
            "83be592ccfd6cbf00f2e299828836ab6a92afea6b7b32e5c4ca4ffc6abf095f9",
            2051767,
        );
        test(
            "f7ffd535353b3296adf5b9264b26462fb8944ae3aa917ac2454bd3eb967308c8665896082f1b8daf1c352dab97c0158cb88ba10eaf323e5ef7cdd3c5ab1188c23da90a1aaa46c18dcdc5f32c5160788cb6331a5fefac666210fad0365bab019d1e62f3389d23ea6de4bcee0ccbac2ee2c86dcfd446c90d3c8290526b1d818e2c28d12e8f2a09f2672578aadbd2b1b612acaf7e35727543cb14272bc807cbb24bda7cfe6ea7eaa3a0533b0b64976c24ceb855b16929c43238121c99811fdcfb80e7217825397655ada6fcfa4b5a7527dcb154ea905d8a7d9b1f7cdc5ba475aac290d8fd0ffea6c9fc08484205335b78f07985df2537301bfa1bcd96f10cad3053aca2900dce33082b01bd66c0b2d801e17f97ec693b2faffce2660b7b0ef7564d8df418ea2c150187c8dc717a737843bc1bdaab00d3d2820f592d912559da59056d659da775d62d30588a778f454b99cbc50578867c8e6274d23683886869d0655c68e65cd79292893d58a55c6ac1010856d94a7d298c1c2f9f503b1ce62bae9afe5057186a10bd7464eb03c47b0e847e6ef690c248f385a121eb7a6a3fd4e9caab6c0c6ac5d4817b01dc72ea12a73ebd47a3d32f740c12cb076a43f35ffb6afa460642f75d05f0b3ffc8999d4030a80e61ce3a54a5c5067968a113cadbcf1aba7d92bbf0aa672a5aa98dfe2f2638d5fa7b624d91f4fd66b02f7a11e798bcbede2de8cd6332fd4fb4fe6dea91cf1000ef58a9dc79b280bdb9aa178f19f39766c965370879229f1b2151a9029b601060cf4a7c28e506b14a58d0624d7838338738b7f54fa865a4ad3a8ad730bf12c2b48c57b7edced9026780d51b1cf3eb91d36e0e76c746db891e510035089225e75db6abffab62f6072d97f5e0aabf9437171026c7aaec2fd28fee570dcf6b9bec649cb6559f761a1c9b634e926e226c5b5c581dbbb1ef3f5c6c489c6d2e59679b6b8d6d9a2200bd59fe453ef70b84dcd27dc062999bafe6bc856d4bbd5276c683fbeba3e57bbc247018b22711b603d2f9ac18ee93338b4d561eef7031f7b6a76388e40d88dfaa5e7276b25911878467e7d887bef4f83417873a496474021e44330068e8081fe84a1eefa3bdd05adf962fa5ecc907d3d6e1c1cbab1079b5c65a560a337c8e305e6d6693bbf539d18d6efe8777146925741957067a0fff681ea2306d5657af9a9313eca5e879276a92e1dbc3d29dd65874accf34c399ddd2a39c765f335d49f3a3862a",
            "19f361976db5833325604c31f6a3362096606d953f34e78caec6348c179112ca",
            "20731740a7ac3501898a5d309ecb095abf07e4db57ae8ee25083596d2f41f747",
            "170910bf127750aeee7c26b0136f2ce3c5847d0ddc91e5b7c3d1a561ddbee6ec",
            "157becb8920260a38fda21c79285e9c3438e22fb1322f52cbca78aa07a90dc26",
            2014127,
        );
        test(
            "042907fec45e611bae8a453343ea30f01758a717fedf53d522e45ade68ceab671e3918875a01204b40d579672d6f1f2780f3d05a50e260af372c43900e5e314da8e15cf5a6d5c2247ab28a10cf13994a7e1bd125b157c98365930fa76c871bdfc1a2380cd15b7bf7dddadb4fe36bc5ae45f13a3ee128a4f74b9b77501d265763966d9e240093539294f9d7aeb66a87c6021ca2d9a6e0a4dc22f2845e295aceb99cca53ab3e09f72476d988f4824034a334d6f9314836fd60f35be29ee4511113070099c7bf8390af96eeb9a1a820f005181a7ebc33a68724eebfd9d841f9f574d40ec8ad93e3c65fd73f9bc042dccdc6b5f0ec0aa2fc12676baed24f1be7111cd3d221015ea9919434dbd25f987c985a0d4478d45a43627f13419e4d07d2f5c64038c9d5b04e8aabd950d98e2563a68df1ecdf22b3179969022a82acdc16e90ff9d3804bba8e3ea7f8d06a390cc97a823bfbee8096bb93c399ea2d04d592f4c1c5ddad50d977974c14711cc7e0ea339f0469b9b688688080aaca49ea33c545d3941cafc7732e1058616b22df13c9bce1923f13ebce4485c2e8591881f2f8ecd4a3553731b3a800bdf6125c7298cbcf2e060bafc7df05b4c239e553b4c491397d8033d6d3525f5da13c6397e1019cb3799c4521985cb910f186aff6998f556c3c0a1e8d9a47348c35bbec752bf5c0a689a529a50980f27220305da3ef7c550d34be357978c7774077ca623cad9dc04d1eea0a4a6db6db071edc8c6c86d77f8796d887d773190c5f351146c092030978b9ff3eaee5cda8d0715e05a2ecf08e31bc2761f95be070053f999ddc9d6936af7ad1b8177a180132a90e738e8cdc45ed0c2c7e1a4dca126610fdc286e62bda134aa75dd7ef95e5247e6a6999ef7488000692884ad450df70582003d39e1c4d1cc808ab880d02b93a3ec1f784a4ccfc60e584385a0c0e7f8dec04af24990495e7af4d43699f1d8801c4c86abaa2e6f42f3f64f634065eaf5ccee63ed884da857778cd627c4f4cc1c0ac31740731284c851258cda05ad9d9eb55760c81036c74876038f58fbc5033d944e877a92083d95eb35cfd3b480e90ac6da13ef422324d00d7fbfba7c6ebb6c726c1bb9f8be01ed3d340858769c25c6643fe2f046f2ba5761750e5a312ae2742873f845ea59940e0b6c1c0c02bc88a9047a09e63aaaaffb4c26ff416ed086d7ebfea314baec4539eba00a6b11cf7e07a0426590d07e11508116db2250cac16f225368964c632ba9d4db50d11e7e0002d8f9b2785b38fdf1eb10f6e87e6a2ba4d6e0c47c0a3fd651e9a10c4db3abd3737b53ee645d3c16696af0573f9eee37c3b66fc",
            "87f56803c3079e4afc89df7285078b8f3483dc8503f3309470cf09d68dd1987f",
            "88a19559dc7c0f46314cb9fbe23c7ca2946db2081eb2b19a4856f4c18e8ced24",
            "f3651d3fe2e74ee010284c6a46b9eb0b888dd8271d053cef7cd3a68cbb4e67b4",
            "410f675f6904e58ea5c03cb10455018032c27623cf9000a8b8efbb2847c0abb8",
            1981542,
        );
        test(
            "f7704c6b62bff847096e20a6ef8f8bb97e6887168321a1345312c547e2ab146f35e6f95b76f2e8ab26ca3d9061e0da59088ca8710731b458711c3b8f119808959987645ee92db847a4c4c5240e483b1ecc7b59d2967dca862a86c2db086e58a78367acd4579ff1353a9d695f822e7dc3751319504636182974987e0f5d5dc8783705c252678c90679243634d62841e062bee4ec7e7a4966d3f270fe6c9463acf0bcafdbe83e2d70a3c55a83ee6854f4a80a163cba43e479d3f32781570acc5150255489e159dc6a5da38f3e9d8af45bff5022262216533954e2702ead4c12b6d4b4f7bc475bc61bcbab54ab99a880218d2285a8a2db11159a2f8e8234e171ffaba33f4c2d8019c0000c50fd213c5ab40160bc89042708500c52c456d859516b5d31118c570a088db80fd42490703bd752e0ca770f4c471e1ad15855b91c7785534627bfca514fa978bc8e8ee5fe6a5941b950e8e7769e8dc25387f7844f939cfe554ee41b345dbd8dd8a13449e3e4b51f900e53c8ca9dd49d9faa87ba52c60517b8d6a866a8da41c5593dd7dfe9f5e90a726ed07977f5efa12079c374109f698e6f85321a1be2105cdceed960bb52a23f7da5dfeae630145ee61f2b33e3935ae63380f4ece5f6ede54048dcdc706d993dabcf27cc4eccb45ac28cd55ab2bb775daad1f3ef57eee8fa23fedf25e4455c51ebf9eb5610c2f17b8a95daf9c87ea1963ee95b3bbd8d5117b133f3e2b68fac2c96f480b49567e0eaa92f234a2d75917f31de96aa41ee6e591771dbef286db639ba4b8fff112b0d3a1c6d6b3e8aebf2108aa8f97c95e41c3e059dbdff6d33ff321f857fb845eeb32ad6c09cba8179ff8dccc77924605d57eea98afa8c033343379abba0ca666a77e2dafa24f22ba7e352b668d56b723b7b00d0e2a0ace7bc17ec596fab3e64c06fa5bf6ef6644bc2f2b7b007b113f241cedee319009f689fa68496ef6f186c8ed7fdc82a2373d00d326f57b743bbfd608eeca2233b694e39a3fae56a4e2a2ed638e8f11bdb633f6e7ea253f2d24cbb7270202da2b9b65ee97b362701c5f404849a1badc7f289bccc65dfd903253a71c8f0ab1fa6004420e1ca05972d3f40258762722bd6abf58617b9e4722f24eb3d2ac21304efb7fe5861051a46b409d28de0e457d66552baf80eccc72af6ec1018a7e118644ef255aaa8c0b51ed8bb78ab5da839de50c345b8275871a15a342e05fdacf0babb8fd22b502155a4e86146b554146666e623baed63fe99af9999bcf75eb3c9c4d7697693aae8e728b1c05a4b209c025e715f55aaff0cb959a477e16ceed7097968996c02f2c19bd702b32c4a063b95f855ab426b0b643727091eebb1b5162d93f9e770922e4314b1ff18fb6a3771a4fe7ce98df012e0e276bc939e608b71f05a5bf84b2b678d98bf095e99c857becdc4213e3c8837bea456ef29c585a20b3f0753057fb449a8a85c5f3c77316accb0977764348895b5c25ca4a7fa09ff201c5f78987f0323fb73ea5ed79e02f742d277d69d5840152b0",
            "d42c537985350ba4d9c416a411046383d6f57654b6fea6a7d5967296ee710658",
            "d66ed3af79389cf644d688fd978b84a8212fe34c5f5e57a8d993366c7e0b788f",
            "46cbe4a65dbe8713727c1412d01b0b35412d8339b4f18cd594b4ad5ee616f79b",
            "953d2476c2be1329cd7f2fb60a6cefaea6e4a9192491da9420669e60eb49d7a5",
            2056079,
        );
        test(
            "5f63e95421ce62ac207e1239e9e2b763ead4ecd17580d333005e5c724646f2633101f0ba797ae905ce68167412e069763838fa4c5c0d155efafcf021320cbce66df4f4288eec02f5734c866743177eaa18e32a19afa82133921108ca54311aa79cd5dd6c7bce26732412e614ce44d8f5e2867ceab20f61f44ec2a8ffffcce141aea7840924135aa68a14b9024b7726b9b7002baab89763f9997ecf5d6fc2c84e048df8f46c8dfe3bb12b5675a4f68840603860cdf58d1fe56e7cbae7b180e717891974f8a4aa032d89f2771da9b9a63578ea76b7153a4211388a0d6aa27de720100325cfd8a88c480e7e2b67685c0afdda678391e1ccb81936f96bef461131b3f4289f902d7cec94f3cd651b5c3a36134bda12297ccb7913f8e76f0972306a5bf119d6777c7caa01aa8fe8f0173134571952540d0168a895b6671bcd19532fdbe3d6138ac1ea9b7bb64ff951dd9824e037d34cf98bd0e4bc705b6dad47633017afa698dbcc20090d5741c1e02e918dd96979948ddf6c9ab8f889749af54c4f6916c20a74349ebc36f46f6b6d47bf7cdbb1a5bed4fc5b9a3766924483e2e5d837976d12122ad6e75300c30d5a94f0322dcb3825c835167699c2b1be79ad28e1ca37ab87afe90790611f3a6d07faa365dd1e754d50f8c462b8d6004ac40397e6571b5ec6dc86c164cec07f41ff82e6f4e29219eab65b0ccbae12c9fe482f18b8e51564013b1ceb3c2f3570f86330c667e21aaaea2a643500f9d758d533f6582239a1bbb0bbda04e22f749ff436ead6df6efce849879e0bd3918cd6d55e875cde0716e96da93ad752ecaf481ccc5d77a9d5207dc912f5972719e105023cd2ab8ccc3e7c5f38d04c6c37667bc8e2036bbdaa969735e07491ee9ce8191ba9a5f0d084bdf616674a98754b6b78510670c2b74ddcc744071d25aed8a6efcccb3d03be80515344915674ded598c855738b86eb68359b801d61daa1b80708dc615bbbd1051c76ad05023ec48f458403adaa0d7becd3a6799cfaf08abf42331e639012669b0398dbe44e0251bfff268dd1a38c63a9fbb22233fd22eec520976e8a9c4c4362fe555c1fc575d731a1e910b433a3d64cbc5b231b623f0c49a41355ee86028013295b8f1f59f0c3f6abea0887c74e7de2676ba4a411546fdb5f7ddb3d3c5e19e1b881fd7b623b79f9e4bbf43f0bb42d2b195130b4630ede7851cc6fba851b5e7bf145039081e95bc8b2e6e0c73e7126fe734669ce53c1ca58ebc72cc787361993d585680fda60c4158155eed6471c8997fee5259fa3405b10e1d5f955d28a12a01c8e9d04f53ba9130daf49618cb553e0d95296adcd0d4b72b73f0c6cc531bbd1d6f466d1e0f60559bdfc7b3385d671bfa3f092346ca16392871c2b8dd091f6de8ca4188b40dfb9c7ec7fdb8fd1ec83b354a599d917accf32875125f20407bc20e3f08cb959e9f84cdb95e5508b0be175156c22d4ba37f6fcb51d1ec109934ace91564ee52240ac49ad6ecd85fcef7db006fc840fd32f387800446b8b4509e917b5001297ae6515db88d28d5f19025c8370323581c47c8402cb93cc2918f8d93f109e46d92d43d95db7ce206fee34d1d4243ed7e6b49722edc4772f71",
            "2c2447a2cdd8ad2cc9b02a499b2cf42c96f8439b49a52590b9ce44443f37dfd8",
            "d689c14e5b89bb4e4cf64a7d60b7e242747c34b270c292e255d97c7211a44aeb",
            "1f3cd81486cc6e71f6e7151fcedc271b3195c4407895eb09ae3337006aa9af4d",
            "99b2220c7464edcf63925a8ed69ad4fc3d1554b9ed9b6762823881763e79aec6",
            1952566,
        );
        test(
            "fccc58bd0e81bd73e483f2d8b57b750d29ce3e6c9f18ba24317153fe06d2f5a0081bba69920baa2ad4e0e34bc4613644c67aa841e6210ffa7818de3f703cd12711a54a60377c0f24285851aeafedc5b405026957782754f3deec5c81d9958f07e377340b08c7447e1ada8201d01c8fbeb454d131e4c8bb284a69149f2caf4d0a535845ea63f5a67100bc138c3b578cc615105f76946c9eddbb2ea23a4e71c2cfb3e45d0a1eab2460b39815f652d10c7251979abf7eca0519013983266dba8c6b08858a48f1993eb5028b0491d66a32427b9554ae31251476055e912c87fe9e088db85d5610e25a728b313b58fdbb066b14746c6f35ffb35f2da95aef4c498661a95fa27f102d58e07e445415cb54ddbd74a593c02898eb4626524430bc57173864da6c84d4708be3a5006ae06a0cab5692d456a721db83c066cb871c9ea2452da767cc0520fcfb5abc45b365ed06c699178646c58fc976eacee3f2eac2c73769e3e14a989e99e67485b29f13deb17a4b28d77cd033f1f2848bcc6422023f3fce73e6fc41a7e7c52ea4e92691b6eb5d8fe9c14ee0cabe631674fd6e3eaab1f273ac22682c7884aac1026fbc4194a8a9b7134f823d42ac1db8639066bf400880413968f83b124ddfa27e91ec771dfdb31a76a27d21be47e8de0f22fad496cb6cb3b1570d410c461c360f8ffb3df08fa879e1e48627dac97fed6433d493e2c6ad4c00dea2c1f2e35ec29ac40eb7d166e13f2f08dc86000edc1875ef8be03a3c93c3dcefc0f810ba851fe788332ec10a243580ad7442e21a7d7c0463baf803c6897886071aeea69cbdfd5eed0bc1940da9cea4017f7fcf17b2c25177ebd6a8e9c91ef68ce5f356386d2d66c4d118708c097a5cde3c59e939920f8b9e9b950a905ff4099c1319cd07b0bce12d5171a9b0958315489fd200e498a3f4888899c6d5edc7bef03a8572708ff1a25f011eb86a5cf18322e009b0206dccaed059a89ca9b69fbf7c3e30929a9d87943a4ba5e4c81f4d4c9a1489c94eba20e57d5d2f0266185edc7ff292b99cadc43c43af7ae364b3a42178fd1176cb7259a1ee485871bab42dfe255a4b127211831ae24c1f2976b0872459813f5ca04750c384eefbce0da48ae1a4ab6deb87085fab8e87684b76a51a158855dd5912f7a88e806e7fffc26ba0f80bbe65086bb15e72226eb3cb535a8cc3043f720b7f9a76fe7923fe2810b55524026799bac74d7b98c0904ef6a50bac30b9cc8f89955f8d136c0ca4995f26c6436b025c04a8fb12e77fe742b962bdc447f0959233a2629799c441cc993f7790c208232c226e3d4e475edecb0c8cf7056eb40b35b2efab1492cd8c115c171d20671e7f94cbe2987efe03e9f402c08dfb1436e38ab73e4545ccb507585b351c77059f3c68ee0a5f4cf390e1655defb4c610da4c3bf5d311267f3da3f650912d50aafc4d4921e0814b8d5a6f243c99cad3145e326589d990223693c4e246709f720da817c22dda18f89261145c19a873fb51cf82486fe3cb8b6109182acae07d33d51932bb68a3befa2ef826cf4e9462d69943dc01ec88ee323f33b92062b0af4ff7c2b890ac62cd7159c153abc330716c26d7bf86d1d37a174cd6730b6c72db7f2df5d2303bb6aedd4ba45f5952ffa7992eedfe0451059ac079b4882cb0a0dbc374ca5b5aefcc435e46bf2de0dd1666142e19cbd4b261f424c533a75df07a90f3cf5dd9ed9bceb90db2cc731d34128e34b4940b7b2d067376e6a25797faf43c708937985de1abb3baba99d566daa75670b6d3a50254b0abbeb58b5ba29453ae33d1fa08d989d6c3e60037601f",
            "6f8a76d9587fa93b25e384cde8311a84e5ae6fca8bd46354949353fc07c779c5",
            "6dd4fdf7fe8582635559d1602e89239df518f4495306548964bd0b5eccaec1c4",
            "505a269e57824be55f6936360d5652a112c06c38de6635fa4c6a954f028b19ab",
            "dc96de47b8876924bdcce08f6a8105e36b90bf2a7ec21ec581dee9f3062bb43b",
            1984075,
        );
        test(
            "53b6c05215001d094d1e4269e05d98bab5b102c557df1e06997aa9e6b45228dccef00346525046bd2fee620d327d755dbf64d7f6631ebcdf6c8f6b5e148791981d51b630bee56b9ab69a18bda51419696433d83ca7921f844884d7d8b8d3e945ef2d59d833aeec84d405b27e6a857794711a5a177ed0d36ac7dfaad9af9d66c3ccfb861fb4ec2e70f2bf2fd9e3022874706c17e21698a9c3f8a3714062b7168c536cf01217704061a1eea3444db53625c8544bd8efc3bf981c3cb76e9e7a4c3784c889ac94b7ed1c6c5f72b847863f5a07b1afafbbb34099cc74d649c3fbe9c7f7032da8a144be54eb10bcd9f323a796d217b9bb38722ca1c967da61597e28122b976eca99e84b2481c73dd71d19b49dda5a510c70fd881eb25fca8df829d523b384cf780c8be5675e2ba804e197180b1d26f44f0ab5c6945593d6f00ab17427df6f15eb61cee22a4c467baccd8c5ad984d7bddb4f8f61e342af9d80674d722ae648279c2f8be70af375f15d704281d3f11ec20b8eb494479bd7de2358237db6bd17e65535bcc4b0b2f869c328c180460c279d5e76b4994789d17164e4c7ddff22eca959a939398524a927adf71704cfa973e68da45757b6bfd7762cdb7dd37aa51467d74973d81063a4e3338858c62bfd84d71493ddeb00e93e6707ace0641c695a20b6785f537cf4a38ce1a92b8a453a53ca9c691c7c26182833c9336005254d789798509ef57ad379206c70e8c3d2dec0c63644b6a994aae7bd9b37736be8384dacc75cd538fc508ad277178e59c194589c52556d69173812af631736572278a2bbc234d56c6260b45c111d94c4d8599169dea6ca7dc42942fa1cbc0787ec7e99156a28b7e76f7be17e6a9906fef8996e550b1ecb5cdd79f6e35c955cd23e225bac424a3e2949222d6cfdd26a18f623ae6f12e80c93c082f5af8049067906e06bd6c4c8d314b9e86f6242a80d7fc04960501e71215d06ead949530a7dfeef07a3ec7d5e60ad838fac4653b063cb94db33accc5e617d0c4583ebe13c5bad45f3f5ebf89e8aa5eb4ca37a030593be406c9ecf323ccbac9fb4601df6059cf2b68312c8d6188d9b68bb80391caf4519f3c87e30be936a8d60b839ef72e9c0d8ef70236b0247aa2111775d0472c09e2a0a0f66321cf8b8ceaf4ff2459eda9ad086710c7d251f513006e226d6d2e596077a0c20c5f265db081cbbe1e684cffdc101c64122a1a3570c6700f39a5bd84814e057c7f583d3d26749ead3605be0efd082bcfbb942b6594d960ced7094fd846dab043e7c2843dd4c1d2897c693a320ef333a648875c2b7cefce9157154725fafb615574b4f9d26d1b6d0b26c06d8db338fb5a625e8e54410e4123b57b829b58c22f51a8c13d3130449f17a419b34ebc3e7126b25a06074ecf37621e6091c0bcab3e6977d968081186ad8b0f810e3727b8c1b300d38c597613feb7da938cec8db8fcc3d65465f4a21a00e76ec82e52b487537b77f1f29857f087b44065d7a7fce12d1eec4bc9e3be27c939e952011aaa3c2c5776662cbc109eb3acff0f011ddf7f5daf9eaaa145def628cde1a43728296ff5de99584ec3ee11a9caac352daca0a48538d14203cb5c94d4fa204372257021ffb88f3ab20c649187588033b3af6ae22219863b77989cf3abd15e83c0a959c1fc9117c98784d89dbbf2b3b1cf7c0473d66c156ff7d050c1856b3cde628d8ad6a155ecce4eba502657519f208f84353063dd90774b8ce763372d833cd04cb4524df0e43c62d235e254a20f2cd30b2faca5dc26d67014604d79f80f4b7c56322c5719feda03725a10efb88b398437445e55d7b29dc9d9f23fa0df236d0184085954c6e088ca23af2f86368c32fb7cd332b4db5586ea17f554d9f544f5609e8cef0649b5feb48e3bfb16b9b584db98441135bc8781d17a22b3c87181c1e88",
            "b6c81e5676e663f6099af4bccc6581346bcef3e5101c2be5b7f88c2138126b6c",
            "50920f3d5301612bdf629fce8dc1102b7d063da89a5524441f732702dea35842",
            "89eccb638ba9d300fdd97a5688654f63d2aeb75abd697af9d1d1c6ae5d7d26ac",
            "1e7ced5b56c2ff576b7c8d1785b425e4485ced977bc5c4e538e7c5c0584ed288",
            1926467,
        );
        test(
            "fea0597851550364fa6f1f84e1f3200b947e560d8813d99c241b156b0671ba7d4282bb0d3120679108af6d35ff9508bd073b915e7a1d340c34866821862386d00b6cfc49e0cd3dfd20d57b21290b480ffbb16dae66b5342faa85b81d6732fd8a39449cbcb5383d38117741e07dce5b8ffc7e3c23900dac47a70256a26e064b409d441bfa31623141a516839c3d5844db9095358df83b548a3c99d7a554a05f343675fc4988c952c369873de30979d23b2635fb33cd2149e4399d8e63d97cf30ebb7d51d95414939f7cb59398fb5bad7f9167f75f2b3a58d37d67d02f3199fe9812b15085411c1cba3dbbef8e35298e2a4f47821c576d6bfac422c1d2e0531c4062cf47b3e8c7ce65f4ea164fe64c60925f05c7ac817696a199e548aedb83bda51c2378137e8d50e45f44a6dec5f72e57073c2f7c4baff0e29e126ac1fe46083869ebf11cc8882a0026029fbe0132cf467368a9f146fca65c2b2fc3a207c0fb0a3248a5336193e9beaf9f295ec054e306e7a2578da66f1304a408724c63b1d99a2bbc6f02f70d56f321e1cc0e9e6b03dc153d9e92dee74b076b8a601fd10565fda9808f657b7ee99332dec0fc99a3b305cfdd6b75b3c8233a60c9fb5e9cd65fa63d10092fa719a9aac13df4dbd1e6fd2df972ce390fa5121097400bc78bfe0585de549602efb66fcdd51605d22613f4e330649eb21c71aa8b11733be221efb6d48a0a7c2cc5e334eede7f23bea7baa2fb2d7b230bc8971174a9adb7e3eb4faea466c78005f654e24782466af155a137d637ba33448eeef4fb297a861c3b2bb42cd9e28cc35c0cd3cb57b570c99db65c9b6c7dc321991b29d9362eda56e1a347f13027367d9f29511bab14a3d38a4cb2bc13b16e4cf0d8f9cdd9271573f9f95b60d429e72547740061de74b4956efcc9fe0f3492b1c59c4535e6d6d63334a2929a12e80c555d3280aa190edcd83be4da8930c21c302e33348aa3e0072a551b80b5842b06c3a08c4a14ae9554f8a61497f40288634d969830b2f3824425b11228adc9faa055d5425371e21fd7dac9611acf9e881ebbc7e8dec3c3a4b4c89de888e58df5c5bded2808c91b2f8a2307e335a05465aecce38f00261d2f262942cea01e903240b1f15cca9af24bd115da73d71acce5aa9bb87179ae4f1f1167afeb103740a4a2512d4ad4e9698ce31d0a1d4cc6045e884408f564a01d9616db0452a7df3145aee000ce2c8b472bcbac63b142beb264c868a859effa454b83933ee81dfd7c4c841a1c5c00a4f811011ece782e5efb00407c7f55b4d1f01a8b9776e3c72396f1745b3b8b8c85bf281d6d43d16b631b8f780e041d504f8461711db8418eff66f34f19e4f65dd75a91752dbc21fcc62cf82fd15333dcf22adf55657999b07e0fa31372db80c6adc5fe5d3185ec2c6ebf17ed78fc0eeeaa3c24ffad20bba6a187726b322b1c4ef3c9d816c985c68cce3ed4559d5feeec12b856abb660cf78ddd2669dfb57f5a56f2bf5ab7f735fe55527f60685285318ba71caf56386218b97b978234d00bf7c2269a8c37cd1547601e1299a59d094a9ff748b57547a7a801452a1014f113ed91696a2659302db83813643cacf47bf1db5eb4fc90695310ef068a997a5157101859afb04317c9a2a6cc6de0285e22b5931a4028e7e03238d2622ab30de3d0124e94c59b72247a2830e93cc408ca98c75091839aa2d717f6d5ddca9282360cc7652261937a2cf47357a1ffdd0a814699c449bcfddf0162c2cad115ecac70bb2b7dcf484da9034c454d1267c5f06634de66705a8180c36ef553b8d3a405898052368203a08b6e23e2aa1aee7d2d30bc3802e48923ee2d29f77f1533c4ac2a61b14d5c9564582d2aa3dfe7f8d7af42344270c960da377f4db95def703bff359b74789dfb8624115cb41e896f9184327c7e0753e24b77f4d1d326c7b13c4d122abf430098ec1e5c9ad35b86b3688a32c5fbf203cefeb4cf3f929d689e2e193f570f219126f01165ba1c44689fd236662803020c7237c9da1d031d71786a59124f48eb6b601a5593885f0d8a32dcb9616e336dab7fc4db2d42958670604ffb76e27f5b8adf66c6f66",
            "82d2dadf5e7b00fe9332c9c8260dbcc290cbfcb05045acfe020074f6f9c86344",
            "cd473d1f6ed1a6b8966e94782dab630b796d429db046762848c673a799afc830",
            "b5276f1c56f17be7f49d6cd76decab4fb42f8909d02c4a7a5bc271f7563dbf68",
            "dd8046e7347648d685913f0e160714e9fb79d5caf7a535f6ef28a07b92ac963b",
            2017232,
        );
        test(
            "ef5ba0f749eac3dba0bdeb602279382971e858521b2fccfaef96bc67531061ae411ed6f0e50212925697dfddf5a0eccd3bd53a1082d3590cd91404650f520f26d1243c9a3bd79b4f3d2c8d2db2c4e4c9a441063adde121cbfd2be8ec08de550149f594cc1f3a17dc9d96ea0b919aafce9495caaac0c67434f924d9b94c0e2a5a5de8d687fd6d5576befdfc18c7c73dfa82f5417353b8933bbd1604c96b88947461fa3fdf46a251c1de3d4e7dbcb64748559775eb49f7cc17d76346cdb3f03fb798bc7569f405ec0a38755376afc891fa343ee4562095410c3abc2d4b3c07775dc0f5c0cfcdfa7feb3d9d7ae0662b6251a48cf54e3b80fcafbca3d0546057563cce4959be9094081776d535f10136143304d121cb3f383c006b1e969e6ae06ddef20065dcc8d1164a3f2bc965ddcb859774a0a8756c5efab7b8c5621b7a2683961dd0e2d66ebaf4087037a98f9c25ce3af778b79f78bd750609a7306fc7cd978169752918d32a48195eab0001f97d69f3134d82147fdf593c1305b895b830d19df18ddf057ec7ae5316616aa8fa722026b554b56277ad9a39ea7461d4aa2fa470a1d10410938d258925bda689e3d91455d082c6cc116d068fbb041d2fa12d9f98ee298ff7c7126ce31840e3c6a812738e13a4e67fe868ffadf29bc3de93d1815cc58e2837432a56c76680919caf86335d1779fe6233730ebe2062ff97a0d7a2361d7ad223a079b46af26d9a27a04b543ceaeca532cbcf41959e052c0227751e6cc359c95375c0a8285d5f49da8a6f8ee30621a53ac50925a5adf9f4c7c4661ca91f796ddddcbc35112ea764f0e1f30ac09aa07575611b4693769cf3f6bb7d720cece80b367e7626a51cec28baf74408cb895b4b8dd3377fa66d851123466d4feba84dbb3d0352f74fb42e0cc3527d542b227cbe322fa9aaa9acac233c852cd9a0cf5e8b2de0201aa0cd5e1331dfcda37f49edfde0d0d160c105f9a9a103a83c8d834bb30e64e938e0888de74dcf3708912d264f0ee879ec5ecef7d519139c692a4eef4511680958364dfde76530a3b025dc8b0b38f108baa90ef38e6c865ff220ff8b758bc3703f42f02f024df055e123b7031b9eac43d5586e4e56855c3f6608c07def381fd6e362f55dc9aac415ffd3b7cc76e09b674d79a1d6083ff14bad2b3486c767c327cfa2fdbe82c20ecef086a0f8283e0e097ee8daac5d3e26ed1e20a1ee323de8a9c3c3b721d7f8519d37ddd1d652a5a2f1b8dd2df7567ce818d86854c046637385eb7cb109edf2d657f2e53ff7c262556938a67d81e856859cf9c03f19ffad40c786abd410ac8017baaa4aff0b0ec9d58141c792dac57fe001b2c8567d2add35eb6c296de559ea8a1c60bd0be8e1c84bb7729d5d220d563c7f50d05832ad7c7d372d4b379e691f76d1abaeca24e2051a2caba32d8637f99260fd35d26a916ee51c23a9d7c6763e9b20c29eb96325e2492710ad980c298405c65b985fcd3b5c39faff903c828ab6d503518ac7a007c218965178aee7a4e498d18386de315d29cc47d5ac55516bebb695883482032331abbb5d34bb50ba6fad5a5d04f052e175a2a15e2f97b3708fbb74f98a6ef890a3c5d378083c979511ac19746711e1ec21699b8f35b11f9331be5e05aa810a81a58e739aacdddc7af9bc166576fe190b6ce6068876654783cc7c61ab7219a8ffb41870802397b88ade1be45bec664166ad84bc6f220a8757e0429317a38f6f12e445a3c42153b6daab1bc634d2f37ffb79584f679acc236ca57f6eed67d52daa829b03d5b736588a1b8ad39a701ff1010a35f51ebf9df71bbd1319c8b4a787e932da697a4d768ca2eb5a302881e054edf3f939f35c7e49fbba7c7c4d0086352c113db982fb03920ba47aec5841d719f844905a8f854ef7e8d56db90c53fc27a4b67e738a845f5776936370ea9297abcf4f0910e58d5cb575c8858affd438b9424a0a6711178dcee3e95b09bf1614201547f0c3f8c31691df62506ee310e72a4e6c509eed692643eb720ce99378242634f0f0eba3551f226c07ad85771ce766ab44a5b7ad833f32223ba47d4a0ebcaddb9c424cf322a8ce18cfa6e0499c9bdf27099fa11d2f826f3e1d767d28a53b1593712ce571c3daddd4b2ac116767fd4c5f6a448d927364e7af07ef01c8e901542e1d71c45128810e44d569fde355e7",
            "b917d264bcef07402a7d292a4239dd9f655c4c8542e8836f6b2906b6ab47cc22",
            "018559faf93f9ea06fe27dd435a5f24c7377380b0726fd2131e49f8a7381e94c",
            "056b45c6a93570ebb345d2ab89a85d0191787a8ee6a91b52b1e04a1558c029f1",
            "8d23a8b51eb7c738b4e6028ef43cbec3287ac96a1297bcff8f3b88b7c2b059d5",
            1966223,
        );
        test(
            "8485ba6b12bbabbcff7a44bff9f1e308a1776e256b4f15fb0f30f596a10db307f9ffb2007006430ee984050525b2cb43c0993c88f2834f462c793dc64aade4a452342c898320e237805b63caa35b4810b75f81052aebd31a08842b86aa36d2283d2032e334dbaf76dbcc67624b507b3fbf466c2b97ff73f96f3db6da969d042a79da16208f8238d02f7aeb40fe3244b30d8fc0182a32b5970838c2ddc80978e142af2a7f2cc6c6a8dabe0b30067485b9970414831475962261ae53559a5fc54e72817a77141689c56062df0658781ae3da7ffdea443468167a07a7e93e18c6f53fcef37c318e1a02210bb018edf30ac7515fa57b838943c0f2282f48a2561468ff952ee778920f98a9b19351f9fb519ca7e2d24addf4378adffc52ca71722f99dc3097fbbaf138707c50861ca01a70f9f1ed4c6559ab16006b77a84cf4eafaa2d734f5127ff73df36b2d9c665909f60fc89cae8008ce93b3bfa13427572cbf95405596b88c7d9c05478a501e6a71bb646c25155fdae5dc959e51461d4de448659eb43a03e72f378f22ecca834e410acb4293fa453cf213396ebe203ad718efc0f25c3cd6afb844300cbb61fbd64b4dd28eb6cc299a494c28416a0d3d1b8a493a55285b8c37b789d515ef80696148c1935aa46f0b6f80b3bd41decb6d82db43f58a6dac1870ac8895c2c281958e465dd8bf02c031f0e48e91fd1c7eb6e85453292c48771b51ed857e33eb8fa4b3f49df6704d8eba1c319bc675f3d9a64d192aadfad6ec755af690bc595bb4f994bd64bb5c224c24819a24a0818747d019fb45234d950c5f1d2514525a06d528b9c09b9584a2a9413b1f3bad0cbd42ee9a8a3361153323fc5d7571197a9ef5c68d485a9d9f24a97ea79f5fb44d4b231325651414b12b12bdf4b82ed7f350109130c9de47ff541fd0aafca8370a78b844d7d9e1776f5731efcde4e7dfebc4513b59e00c6f2ea02ec05cb6abecfe104d2b0c5a8e57f8698590c5857acde90b60b631eddd1185824535e7aebc9ebf1038e6645a2ef0b50e46caee5cff23d0f264660610bb5f2174e3f34e274d8af1d97b019962e6e22cfb2c3c317982cedfaf491d9936f32bb16d5fc9b99412ae1172e43f7fbc46cbf0495572a631fc9cb91b3560ec40642b8912d6a110a960943476fdf04a3d24a9a5e75feea56d638e5e06ae7fcd204d165b3fe713769e3db9be850228c36571d5e8b4e2c77802cfbc78d206d106bcde289923fc9cc8032a585de8b35a20527eb8c51e5d18aa52b7c8f54da6803ca8918a18f2371c5a4f8d64e1181a27dc6cc3968d1ec4ec5a2b1d785f37b70b0285884a69cccf743c55c8828965f73885e1e242103eaaf921d9d13513641b7098accc325639a56e09d0a218223284239d4395b78f66067931d527665ec88505ebcb2adee9adc2000a487d4054a73b1fa49d4afc8d54763f7f5eb6e037d776640b69b92cc85571788f9cd190b2116106acd6bee16d1b9c348c6852ecfe778255a92957ee8b777777716a2b6ec5ea745595bfd6a2f9086ff1c76e5f20a45bc035bae8e28ad035a6c896864f2f9308b1a97ee0d55e20950c3154dd71170f93cbc57a3055553cd48fadb555f1898955ad6ee2b64ed5a97fd392da46995cd96c22e35fb454c9422d8edd35f7413f4fd4a908fcedad250b7d99fa099eca10a5a45b5c20c10be18137dc4ad06f6dacbef7716e44b0c67611d6d31499dd3e4ce3f5f7cc14ac02f144aa9505e5a3ea68baa211ed85ce2f5d64776a660b37111b3954a35184586f54dd8cd41a323d7254f28fb915be987577dabd326f367ce41a86ec425fcc3ed727e5133f22f6166da945c6efd1e22b882b31639b2533090fc88952ed60e6566118ad7a23ec3de3b2faf7b74fb59a99bb9b804114f3302be893b91a51c449e2d2a0a583c710de85e95fccf8fcd9b41150727d412a0bda7d646b28e1b32cb359502cf7c7343b4dd5d8cba9632fc482e38b9340efcdfc232088d48587043d9b345e9a3fb7281f00a89555b664f07f47bb7930d50d3189ee25d611a1d4bee675c9ad9e388be51f67d84365639af11594a1f1737197150568daf0ae6038cb6efaeceadad067a7efb1e0663119051ba92f76dd5089927492a694dfb5b951b6d7afe51d32ef6c7c956cd3bf19d3a53033800fb62c60f5892f0b628930780c7332c1d8f1726348122f3a1966555c3732397b1d6994e1c9fb9013d6eb4f8c5ec4eb8afecc6b17aef75fb28631a2aca4f87912f51db46382614fe73da18a6e624d7206444e8e03d28bf289c9c5b6c163ddbfb9310481d86d692271081e5202323ee6553b8c9be94e140563c2900198d8a23751b76a1e1c7006e8c7c41e5b147d",
            "037b5bda8b6d9deccf0ae09c3095598bb91dbcf87ff61a447799161444f690c8",
            "c4e1c78612dfc921bbae49555983c7864e14a8b26267ce7564ab5efa59cb2878",
            "c9e58907967f4f9c897adcfb0f8178891821f8b60e0e446c66a3a46f7e780285",
            "048ff4fc551455e6f3dc792041f466fb2c58593b54119e80486418c281ea2fa1",
            1909909,
        );
        test(
            "3401683eb417146f38963c6dfd678d93d5fb4f3e1fa11526fcbb484c3cf2b79dbd1a220da3393a80a45debda44a839efcf5c66e94875c2dd3b987b5b6028bddb6ff3fa0b57b5ed9231452ba54be8a0dbf2b93bd973b5883c4c4d0a7f2a42b14f25e988515e6f1f306b9eb1d457e6162ce02ec46edf04a334cd4b75dc2210fdafd316dbe39a6f45a4e38330ec43ab366baf3b2558062bf7f1a9c7f145ebae00bab4346d2e685228c5865f1cd33060d90ad6721f37982df35f064bc9814f614e548f62d784d0c025d4ee37efb59bebca5ad6741770373c06ad176179f48a46e279e56cd5e801ce29aa2b78e040dc53d9be391abc4cc2fbee567088de723d6288d7737ad3b82fd9489fdf151fca8135af88b05a4ed431408361e0ed7c52ba6b0680ada3189a470e23b71993e059592e9efc32817158f7a94b62652d4cfba4f700436e01e83e5a99a1a43af351324cd3fff7f90eabd805feb9f41124e533761a4f49c6792c50c2611d2134aa249bb75094d9b0e95b4eefd31771025981000a3df1f875b1b04a4ae161c6a2540c5a21b5e5d134192f4d99acbaf58fc1d2c7ea440266db66b41efa328565205b65a2653cdb4a316a133b0dcb6cedf79f2fde9c0fd8bbfb1946f60cd426c677a2cdc4a79b267d70144d9c5a98652803edc3bde7a17c97e0e9ab5ef96e283045b6a9db1ad3ada916e34244bf87c4a8d468120df738fdd2a827530484b3e64f4b0baf1efe2e3d55b69418d55188e57521481bc0cdc763e4ab4a2bc3b5de426f6dd2a2977d08ff2929a8919c3345c2684e18b25fee6eadd5ee0444c2316955cbc7ec5844767ac1ea8ba347ba22d1ff4ba6b40ec36c3dfa6b488b6a3372ddeff0917d1ef166ed94dad6fe46ef3947ec1185e2a0701d537be274ff948534a8072f8e7215ecef72a41eb17b043422330b9555254d797b264baebb805f26d7a4e121f7bfd37517026761642ec12120db610a112ef527d68865ad89d8ac5ef053c99fd224cc3c2f9ee2edf23e342ab3891d5d4aca849530cf3514139dec39527dba24ca3db8ce158512a2b514eaf2bd052a5c9b3fc7217db1d6da409c172bfc60591401b32f1f51962b21227ea259dacc18be6fbb4f2f65d845c8d6445c63d9789714bf6cb2ef8ad6c5ac26556b3d23566e9e9c37a490290b8a452c25bdd7e62b0ffdfc80e3786a728d7ea5f44aae7ecd7a696a6ede1928a0e46f29b4e5c85ce1e9e26b40f267dc9706550883e3a331ccc9cd704b26341419e605365069e39ee732a047fc40406aa69d306439e8d8095e12f371f61f161b46e1b4ca3ae969daef4ba39a0c938de34b1056fea27711602bf17696bf7b3763f9e70f35a8530e004b398bd950cb1ca423abb48c8f91d469cc776cf2a5feb9c888f6aa76615402c1f401acbca4192b5f6973ab5b324cda5a8c52e1e0edb4e1b94fbc5a6782db4604ae2b01f6417e9d0d9225c0f49d4a3a4f59ff87cba68a95941ef6c4c55643bd45ff7eecb75c12b23d223dafe1fa595be4b7c1d439f126dea09f9937feff9ac6decc90f1171fd47a657a8b6ef3032f486b32173cdd8ba7f902d8c80e60f9b4da5c01b613cc6ab4205854a95b1e9f36dd66a76dbbe0fe8bc97028aff8da8e8e2846867037e35c18ce7cf9046294940b89ddebb4c8963b17f6e43552db85ce0ecf2890dc17f49c858670606a3eba4fe06f90f342794f50e52018594ec19e6a47583510fa7c59ff6a498d41335c60006c1f5f59fe84d73559bc6985f6869a6c0e6929c3901f72314b66df22b3c540a8e7b3a2db9d61060e042cf0cd1277297d7b549bd9aadcc2cccd3bde7abc546a8b47d3ee427aae9be849d4e7b0ea5fdb23eef1bec230a6d4961d229b4d4ab8d92f75df87b7672e272edae1709db31424f0d2afb809c5f696898a944694f6e5e42c497fd67581d9f768ddeee4d5ad2560662da603f7e3ae8f6133ed58d2724eaf8f729248b11f42e99cac82625970af71524545f07b13a10cc7dea21899fea71e45853d828fd351834ad3e12fcc33f6bb3fca11e73f012a3631e4aad93b8a091cfa0bfa3f0178228de9ef9e5cf84d1e0cfce439e0cb86b4b19b7580a83ed0eb07271407f6c8c336f461989d901ecda9d2dbf26e4f724c6bbfbfcda49048f7bdef5e062cc3dc600b94a833408c9bc98ea06d68435e719e2c5b13fd6708ad69a92049159bece630b8da3cad69c9017fd0e7b31b676362194f0d2f0a0c48c3a3a74325a9381d811a2380aaedee67226ac2f2839b5b93a77615ca9c1645a66c9cd44a5813610587eb93a5c0d15653d76b2bb7db5207f5a2c528667a024d2ece1c1fa337a61398a783f2785f8709c307fab83a51d246a3da9de4084262027045f3498ca561c41e373e0987ec2d69cad17f11677a663e14b600a4b5fc22ba967eac838b0ab741b75482503f81a4fd8e5d22a358f6268f976bf32dc87ffe5cf53e04cac02999ac1b01831cc4501d695d8c1bb71245933e994682d13be2e8fc1ab3cc7a",
            "ee7fea6125131c1ef85ae2e02099d97a312e0274fc2e8daaad90468a12893ee0",
            "67141c9a4eb517085e6f69510c2af038b5fd1e8a9839bb38a64ea16fd80cf6c6",
            "2e41c9d38220c5472351c4457771aec372d6c9b175a339af8b0de5e483b9f4bd",
            "b27a43a7a854e5b5cb82dd113e8212ab2d1d20e447ab2ac668948bff99d3d134",
            1897015,
        );
        test(
            "61d91c2cb6c51f6943e05c180f07fd83f1659d9d6dba2eedb34bb9cea72d865f16d1041ff034a307d33525f2c9cbf2f705d848596aee587680650c8b7ee47ecaf206da52940895df702397c7d62fdf504c67fead6c7d1e8d4b3efd66a1521b6f098d1ef3d736a4c4086e3d7390ff0d9db22e72226ebf88d66482ce056462a3d6f04e8cc05efdbb7808c1dcd8fdf998e897bf62af1f8588aab703bb83a22d08e82b35e32b08b0eeb82480c395689da89603649eb29d727cb82392b32edaf622c88c03ba841cc7070ce70c4cee6bf23f91825aa0895889523f907acede44b6f8e7d52c51ee88108e85442d0373a8afee9ca2802f27fe0dcbfb9f312afe9aacc01d12d4e3ceb91a08680bddcbe95f3871fada15deab96fe0ae5978d64c455e47e9b75a5b01be9b1b45bd062bd055eea5c865750374a3cdb77a6c880fb62dfda093283f5586c0ed63d1e056991748fe143466f6d2fb442e2ad390797f90bf46c36e35ca87ec2a6230cf0080caa335485eaa64766677d6b6677bf8d50540bf942ede603093fea16c14ecbd02da626afb0fc6667d7c0290c63f0ac34fecef58d9a0280f5a3346a3319862e871355d7047c7f5ed68a6bb6270e4972e14eb4dc03309510f022d7f0cd11bb471ed5aa217018e21a883630fed648eef25f5299c37d17c76479e715b065c2fba2b975321278eb2444497a217c49a139de95c1d1f5854d798b2da2ef55617de9e80b01fa8cbbccaeb472c6c763645e74a73a0c79b6e022d1003b7d7934ec2767b99e036d8464ba3c7a32b9657281850999535f0179d7211bacb9a9512d39fb05e91c74d0d71cc1b7d3b0d8936499be5a4e8f0798539c7344073bad3575a5602df7dc7bbb8a11b1bfe0a63c2894d409c74e0ebd1a98e77defd3112608d0a39aaebbbaf33a95ee638af8746ef9e75ebbc1132f9a98c0c660653c34b6aec8362c490acb3beb26f74d54a28adddd4c499b74dc59ba1cd553e2757b9a74c701d1a83ce0894d034526ff696810bb25da75d4397048dab03f976a11ac81716c1e4155003f0a91a58238a8642ad9927ce53a2d8cbb8ff143e57792e833e4e70f322cfe5e8756be7631df5c5a6c9754734912f3830b38c5a751b7a2cc42b803a37c171753d9183bc9048eb4bfdce4102e5c24e5b2645796aed7e23d998e2f81ba6d2783820d46c31e7385bc44bef605042cad643321b664878b2d87fff42bff80e231eacb2349ebdda5a2355366c9e08949b0bef928fcf7d593889c289529fc58f6c7f8c3fbd9e95d348dcd7ed386da420519ac62785dbfee9909760070163d0b84c8f785ea1f402d9e3243b33b8f5666797cdb6a7638b3ecea02c5b686826dff584257e4e182a3181a5b89495690270b163e0faaec5298df565e3cd06493b2def18fe59a509ea850826d931a13ae1b54fe0612a10ab79c6a1b9457c5dd8975b57d2abc72d979336531950b3d511fef096590d408cd63198be9dd7b52ecbfcfc1867f8f473c51a861b6bcb1307989f115880e4f760b5c10b1401b06421f94ec5540683bcc5eec53c2a1644ef5f4d82409674e6e0736189d135b65cd4824d2f81cec11a28f39c89bcb70caa2b206e4867172db67445df3df49edad0832759b81ee1f53945b7e68c41a783b0414c23fa61114a597fd6d4f4d0582aa1d104f0c84424938fd4985ff763ac7cbc3cf0ecb86a567b4eef0b2066e92ac1bf09ac4d009865d612a11f9169c2d43ba355a19bce0174dcb0ea6cf9551e51088b816be38c0f85defd088c76583b107b00561e82d7674fc544592b6fe72fb51cce354b69623a7c4d9938ab5184c87d740a241d667eb3ff2b331ad4f4ff8ce369e2bfeb183b7e619161b8a76011980d852494053f199b7da8ceffb7da6a068816a15f8903a672161e6276d73cd672cd20931bf68da165033dd975a879a28e4e90bb5f2087aecc93174ea2aad990e9463190c051523c73027325a6c667e0e8ad32f37504053fb0efe9c43f4e55253124fd79d2b5b452895fef0bb8059e973dd9942eaf3275dc3140d41ee82a4262436f2ebe10f3e73cfc75193c82bd9f24f0c16f04930e651fb0c2fae20721bdafb724d54c3f624145c513b5dd050f105b8160511ec79955075b777d5be23a86bb9278fab56ed564fa85d7c23a995becb589761b2f5d7f773580797e93924d6f0899e47f57dc35ae07a3d9423cdd31555518c9be510b2bc36db35e9c25f900f75c53f313ccd6b84baa932999c27ab6e347ac18706ffcc91ca823fddc48af7e49a61140e032e00347d51d0807470574953dabccbbd589a0f1dab7bce23dfaad7d894d897cb4e83ce508959691e995e082e4d44d7102495429df5bcba0e201d4263c5175d83ed1531ac206ce66969f5c6043cd7b638e8c67a9c98a6242d47e0f941d1956991a96e008c3fa9526b941a9796078cee88834e857bdeaaab3a8d4a717629450e0474486aed6f2b62650c3030f492df1bd91760860ed19074e8b9c7e253b63dd05b8399de2b4a0fcfbcf1499969e777c5f6d3f337047fa1802d4873c1ba17dcf93d0400515ceee00587b87b387a4960698f39ece4604126996e9dad7a20e25cbd9477cac28ffdacf061f1fdc19e3ffa9db8",
            "6b536f96219488c8d63934b43c35dddb87f916356a07327387e8290915e39a52",
            "c70653b3912e28b147f6c726e563798c90eb02eee034f3b8689f7f416b60cae3",
            "441ebafdd34046af65c823abed216fa0272e1cb439f87ef1ac677565f552d75a",
            "1c43ff5c2cc02cb596dec0296fbbb8e687ab63ba540054424e5e6b6c5c7e219e",
            1934947,
        );
        test(
            "3cf47bfba91d45c921bcc188c5f8895d051b65e81cb3b8eefeb9713702e815ea8791ab76be7e90a6d59bdc99d6e653e3ada83fbd209375e0924c619dca0f7596d6d18b5a881f9f02f3f310b704f62d6d84baf79c42612fd0644917c628dac313fc95b6f3da602d8d527186e2dff109a944b939aa69bba10abd9d7398fe0146631e96b38807f26dbc51c904806eaff11fcf007b48d9fc08de733bb6fd7d46781751096206979aea67c2dfa27f3850c05b75e4f41a7304a71847cff1eb6173f555a7dcdae194f8a5038bfae48fadcde0cdd3379adb5307d18e9212d8ad62674e21ab61e794b514ec7766e091a625acdad84f981dab19f8b540568e254fee6d3124628fa952c2abc5397259b048a4392ae30a529de89dd7309ce11f8c1ee3df7a1255bb60d1be45452cb6a6c35497b3127084d3820968a884cbe6c875044d17e8ab173fcb447cfbcca4bd9fa6e9af175f0c8632fcd4488cbf035aa4d218d8b6fa7431f786969404b915522174be3463f7de6ad107edf60ecfc9f7cb857bde12cb37e0a14e9af6f86428f5531c323c3911b08e98077f7292a114287ae741b6342b814fd800f321f437f59d9f2f4d21b32a0507b626d2f436726e10aa9b4554ab426863264ad2d9009f2bbdfe5fe04108e645c06a550e3061d1b84178c6da1522e2f6dc5cf308ce0829600bf03a6d404dcd983fc05077d10b0fb4f0ddd3db7332f4fe7ce0bd9c6af6c9eb5bf399ac55ab897a418370685472831a60b2f5ff1845a1ffe868ce5eaa88ed123c030f0a24c0194765dc113180d2eb2b3267b0336ec1baa94e0133089730c2580b0a97b08302303daaeb878f9f9a9d178dc4892a125eedb5f384e351fce61a7cbc1134d36b254a4a3d9a40327d0e0a67ab1d28256f0340199da6e99ec8ea6ff2f645c771c76e56f35e1ec2521d9b7ab544365c705225ca8bbbafd3fc6af43481dd3d61ce6148d88f43b23b62fbcc90b18ae186c8c9f2b5bc4644d5cc0e2e04c3eb0fea6df38cf542cc14abeaee123c1718e3afc02d78edd04d1c022360891b2a8cb6b8f17f3f862a33236d172aa9cf3940c9ec4d0735d4da4400f500d650e93f293f38558d26f5cfc6a182748dd34bd4c0b5beffc26a7faae08f9776222eca8905e6db3f03bb3e299e20c205989e33c6fcad00eda3e65d0e2d37d92aec69a84731d2b5d1a0a2965dc627b09c529ba389bd38649555844105cf5b063c05ea709adddeba6c3df08c2521109e5d83363c4bd4a4af720c5a61bda81009f01a4d5211225c518f449d40a81da0a689eef64f602baf83b90c1b9d357f1379b69c41c936320ff15ef377242f46c71836156f7f77501f61818d7faa548c14b94a118dcac7f1c0e2188c15b47ca7b48c092c8629f6fc0c22c56ea38c5e54ba4d3241ddbec37b65bd38aef833c9f102fe513c1ff4a59c26ae4e7f434330e200925ce75f475d3eb1884a752852079acc67fa5cd5f855c2b8dd335efe8f98b6bb3b0c2fc529ad150763e562a24308895adc427de32a143b0602bd9fe44d290f2d65d70dcb5c1003f4779d6569e1816942ac1d0680bdf6e791056c8c4023a9f0deaa9b7b4e8a44398dd88c06e3f8aaf591a07a1fa5ec8d5853adb85632457e18aac93caea4cc86d8d4c3e21522b0c6922e9ec7dfdef65c0a8fe70ee221a60532686dc269c203c843b201b57a66627870f6ceb69d09450e178031bbd03f8c0b1b3ada1b146a9bd307c6329a97f9e184a4ee7beffeca483b80185b9263358fd6377502699f2e3dda7a83fe38e4e9a3f7d6907cd03e384977eda1c92bbd9d93c144daf3a1b69633ae4fbf153e1b60e169cd9a371d3223e57b561446265c2b9eb9c30c8bdcd78d213bdebc71d6ad4a6265f10b798bf29d09ec42007258b3e5064e0ee817901673fd17f75332824656db8dbb47a898ad5c7c36cb8e69f9588db9123a45473a92e4ace33d9aadc52f9fe316685b7e9d44f6ce0d098be2bc91ead3b34a291d690a8df94f06344b0e925a049f49d65d6eb668191601e150f05b787c0698fa505e0fa1e4b675456bb71add9460c549aea8691ef9533325fe96094df3892debd064f99293ba5e300589b436373c93f1e104d3aa904001d5bcbffbae627db493eae5d6edf5e6c752a0517a982437c18e3e96a317d3cf4ff75629590c2e29dc5d9d33424eb4ffb5ff8490f0c02b4bdea7367962db72ba148f91f8d49f72100ad9495f11361cd52f29693a0316e8797bbd5e9affca0f7a78b267f3145f4f37de2eaaf189ea7c484323a39aa29dc409a139b7542659732b20562ffa48bbd37a99f7de44fb12204de65984d3a648b19ef971983dfd805f04006961d6a99e7c525a27dac684de4f5da77980f5d91ed7da34b9a2fb5b543185782f2960c58a849255152ba6120799f6a1b90b7d8e0ce1d9162f8a6bb68f9a60f624d551353da2f475f9f6df759f85f68e042662a553fafbf07cc105822dccf8b800919018e7b317ecf465066016eec912f25b10858da0688331c9e31f55aa76b6e338d270d45aaab0be303eda173f0208126525637e1aa7096a330f0d8d5ea159fcb84a1c3517e4796c590bb335910266edf7beaa9eef08647bb35ad56d2b0c7ea064a94b2abea71187bd9ff587a92c0cdd78aa397e4f60bc3245c6f23320858daeb89006545114a52f6f530a51f111814e232f919d071ba3eda1cf826d3e60df4e402399ff91d19de9bec47284c37f9e0886afd303cf763067681aa845c105ea59f968f2b0e5491d18a8bfaff28b09c3e2d77038e2b4a5",
            "fb00c9cdc9f928e39bd5c924b50e75e3810f2337c72380e7a170eabbfcc23bb7",
            "764d574f338fb5d766924371e64c2bd5fddf224d4e74788c0175e720e0d354a6",
            "38be38c47eb6419c28cac8f7b09871db03f2e6cfe7962a63a4e668b5d40b0349",
            "329528545b00249b54ad38569122ce9db9fb5efe2ecbe43b915a4e28ce5cd431",
            1944301,
        );
    }
}
