use std::cmp::PartialEq;
use std::io::Write;

use memoffset::offset_of;
use sha3::Digest;
use static_assertions::const_assert_eq;
use crate::hash_v4 as v4;
use crate::hash_v4::Instruction;

const MEMORY: usize = 1 << 21; // 2MB scratchpad
const ITER: usize = 1 << 20;
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

#[repr(C)]
#[derive(Clone, Copy)]
union HashState {
    b: [u8; 200], // 200 bytes
    w: [u64; 25], // 25 * 8 = 200 bytes
}

const _: () = {
    // TODO: create some constant for 200
    const_assert_eq!(std::mem::size_of::<HashState>(), 200);
};

impl Default for HashState {
    fn default() -> Self {
        HashState {
            w: [0; 25], // initializing either field initializes every byte of the union
        }
    }
}


#[repr(C)]
#[derive(Clone, Copy)]
union CnSlowHashState {
    hs: HashState,              // 200 byte keccak hash state
    k: [u8; 64],                // 64 bytes
    init: [u8; INIT_SIZE_BYTE], // 128 bytes
}

// Fail compilation if the byte and word representations don't have 100% overlap
const _: () = {
    const_assert_eq!(
        std::mem::size_of::<CnSlowHashState>(),
        std::mem::size_of::<HashState>()
    );
    const_assert_eq!(offset_of!(CnSlowHashState, hs), 0);
    const_assert_eq!(offset_of!(CnSlowHashState, k), 0);
    const_assert_eq!(offset_of!(CnSlowHashState, init), 0);
};

impl Default for CnSlowHashState {
    fn default() -> Self {
        CnSlowHashState {
            // hash state is the largest union field and every byte of it is initialized
            hs: HashState::default(),
        }
    }
}

impl CnSlowHashState {
    fn get_keccak_state_bytes(&self) -> &[u8; 200] {
        // SAFETY: Compile time alignment checks ensure that the union'ed fields
        // overlap correctly with no unexpected padding. This local-only type
        // is never shared between threads.
        unsafe { &self.hs.b }
    }

    fn get_keccak_state_bytes_mut(&mut self) -> &mut [u8; 200] {
        // SAFETY: Compile time alignment checks ensure that the union'ed fields
        // overlap correctly with no unexpected padding. This local-only type
        // is never shared between threads.
        unsafe { &mut self.hs.b }
    }

    fn get_keccak_state_words(&self) -> &[u64; 25] {
        // SAFETY: Compile time alignment checks ensure that the union'ed fields
        // overlap correctly with no unexpected padding. This local-only type
        // is never shared between threads.
        unsafe { &self.hs.w }
    }

    fn get_keccak_state_words_mut(&mut self) -> &mut [u64; 25] {
        // SAFETY: Compile time alignment checks ensure that the union'ed fields
        // overlap correctly with no unexpected padding. This local-only type
        // is never shared between threads.
        unsafe { &mut self.hs.w }
    }
}



struct Cache {
    //scratchpad: [u64; 2 * 1024 * 1024 / 8], // 2 MiB scratchpad
    //final_state: [u64; 25],                 // state of keccak1600
    //_padding: [u8; 8],                       // ensure that next field is 16 byte aligned
    //blocks: [u64; 16],                      // temporary chunk/pointer of data
    //rkeys: [u32; 40],                       // 10 rounds, instead of 14 as in standard AES-256
}

impl Default for Cache {
    fn default() -> Self {
        Cache {
            //scratchpad: [0; 2 * 1024 * 1024 / 8],
            //final_state: [0; 25],
            //_padding: [0; 8],
            //blocks: [0; 16],
            //rkeys: [0; 40],
        }
    }
}

impl Cache {
    // fn zero(&mut self) {
    //     *self = cache::default();
    // }
}

/// Performs an XOR operation on the first 8-bytes of two slices placing
/// the result in the 'left' slice.
fn xor64(left: &mut [u8], right: &[u8]) {
    debug_assert!(left.len() >= 8);
    debug_assert!(right.len() >= 8);
    for i in 0..8 {
        left[i] ^= right[i];
    }
}

fn v4_reg_load(dst: &mut u32, src: &[u8]) {
    assert!(src.len() >= 4, "Source must have at least 4 bytes.");
    *dst = u32::from_le_bytes([src[0], src[1], src[2], src[3]]);
}

macro_rules! swap_bytes_if_be {
    ($val:expr) => {
        if cfg!(target_endian = "big") {
            $val.swap_bytes()
        } else {
            $val
        }
    };
}

fn keccak1600(input: &[u8], out: &mut [u8; 200]) {
    let mut hasher = sha3::Keccak256Full::new();
    hasher.write(input).unwrap();
    let result = hasher.finalize();
    out.copy_from_slice(result.as_slice());
}

const NONCE_PTR_INDEX: usize = 35;

fn cn_slow_hash(data: &[u8], variant: Variant, height: u64) -> ([u32; 71], [Instruction; 71]) {
    // let long_state: [u8; MEMORY];
    // let text: [u8; INIT_SIZE_BYTE];
    // let a: [u8; AES_BLOCK_SIZE];
    // let a1: [u8; AES_BLOCK_SIZE];
    let mut b = [0u8; AES_BLOCK_SIZE * 2];
    // let c1: [u8; AES_BLOCK_SIZE];
    // let c2: [u8; AES_BLOCK_SIZE];
    // let d: [u8; AES_BLOCK_SIZE];
    // let aes_key: [u8; AES_KEY_SIZE];

    let mut state = CnSlowHashState::default();
    let mut keccak_state_bytes = state.get_keccak_state_bytes_mut();
    keccak1600(data, &mut keccak_state_bytes);

    // Variant 1 Init
    let mut tweak1_2 = [0u8; 8];
    if variant == Variant::V1 {
        assert!(
            data.len() >= 43,
            "Cryptonight variant 1 needs at least 43 bytes of data"
        );
        tweak1_2.copy_from_slice(keccak_state_bytes[192..200].iter().as_slice());
        xor64(&mut tweak1_2, &data[NONCE_PTR_INDEX..NONCE_PTR_INDEX + 8]);
    }

    // Variant 2 Init
    let mut division_result: u64 = 0;
    let mut sqrt_result: u64 = 0;
    if variant == Variant::V2 {
        b[AES_BLOCK_SIZE..AES_BLOCK_SIZE + AES_BLOCK_SIZE]
            .copy_from_slice(&keccak_state_bytes[64..64 + AES_BLOCK_SIZE]);
        xor64(&mut b[AES_BLOCK_SIZE..], &keccak_state_bytes[80..]);
        xor64(&mut b[AES_BLOCK_SIZE + 8..], &keccak_state_bytes[88..]);
        // TODO: move to implementation that only uses the bytes
        let keccak_state_words = state.get_keccak_state_words();
        division_result = swap_bytes_if_be!(keccak_state_words[12]);
        sqrt_result = swap_bytes_if_be!(keccak_state_words[13]);
    }

    // Variant 4 Random Math Init
    let mut r = [0u32; v4::NUM_INSTRUCTIONS_MAX + 1];
    let mut code = [v4::Instruction::default(); v4::NUM_INSTRUCTIONS_MAX + 1];
    let mut keccak_state_bytes = state.get_keccak_state_bytes_mut();
    if variant == Variant::R {
        for i in 0..4 {
            let j = 12*8 + 4 * i;
            r[i] = u32::from_le_bytes(keccak_state_bytes[j..j + 4].try_into().unwrap());
        }
        v4::random_math_init(&mut code, height);
    }

    (r, code)
}

#[cfg(test)]
mod tests {
    use crate::hash::{cn_slow_hash, Variant};
    use crate::hash_v4::Instruction;

    #[test]
    fn test_cn_slow_hash() {
        let input = hex::decode("5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374").unwrap();
        let (r, code) = cn_slow_hash(&input, Variant::R, 1806260);

        let r_expected: [u32; 9] = [1336109178, 464004736, 1552145461, 3528897376, 0, 0, 0, 0, 0];
        let code_expected:[Instruction; 71] = [
            Instruction {opcode: 4, dst_index: 0, src_index: 7, c: 0},
            Instruction {opcode: 0, dst_index: 3, src_index: 1, c: 0},
            Instruction {opcode: 1, dst_index: 2, src_index: 7, c: 3553557725},
            Instruction {opcode: 2, dst_index: 0, src_index: 8, c: 0},
            Instruction {opcode: 1, dst_index: 3, src_index: 4, c: 3590470404},
            Instruction {opcode: 5, dst_index: 1, src_index: 0, c: 0},
            Instruction {opcode: 5, dst_index: 1, src_index: 5, c: 0},
            Instruction {opcode: 5, dst_index: 1, src_index: 0, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 7, c: 0},
            Instruction {opcode: 0, dst_index: 2, src_index: 1, c: 0},
            Instruction {opcode: 0, dst_index: 2, src_index: 4, c: 0},
            Instruction {opcode: 0, dst_index: 2, src_index: 7, c: 0},
            Instruction {opcode: 2, dst_index: 1, src_index: 8, c: 0},
            Instruction {opcode: 1, dst_index: 0, src_index: 6, c: 1516169632},
            Instruction {opcode: 1, dst_index: 2, src_index: 0, c: 1587456779},
            Instruction {opcode: 0, dst_index: 3, src_index: 5, c: 0},
            Instruction {opcode: 0, dst_index: 1, src_index: 0, c: 0},
            Instruction {opcode: 5, dst_index: 2, src_index: 0, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 0, c: 0},
            Instruction {opcode: 2, dst_index: 3, src_index: 6, c: 0},
            Instruction {opcode: 4, dst_index: 3, src_index: 0, c: 0},
            Instruction {opcode: 5, dst_index: 2, src_index: 4, c: 0},
            Instruction {opcode: 0, dst_index: 3, src_index: 5, c: 0},
            Instruction {opcode: 5, dst_index: 2, src_index: 0, c: 0},
            Instruction {opcode: 4, dst_index: 2, src_index: 4, c: 0},
            Instruction {opcode: 5, dst_index: 3, src_index: 8, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 4, c: 0},
            Instruction {opcode: 1, dst_index: 2, src_index: 3, c: 2235486112},
            Instruction {opcode: 5, dst_index: 0, src_index: 3, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 2, c: 0},
            Instruction {opcode: 5, dst_index: 2, src_index: 7, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 7, c: 0},
            Instruction {opcode: 3, dst_index: 0, src_index: 4, c: 0},
            Instruction {opcode: 0, dst_index: 3, src_index: 2, c: 0},
            Instruction {opcode: 1, dst_index: 2, src_index: 3, c: 382729823},
            Instruction {opcode: 0, dst_index: 1, src_index: 4, c: 0},
            Instruction {opcode: 2, dst_index: 3, src_index: 5, c: 0},
            Instruction {opcode: 1, dst_index: 3, src_index: 7, c: 446636115},
            Instruction {opcode: 2, dst_index: 0, src_index: 5, c: 0},
            Instruction {opcode: 1, dst_index: 1, src_index: 8, c: 1136500848},
            Instruction {opcode: 5, dst_index: 3, src_index: 8, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 4, c: 0},
            Instruction {opcode: 3, dst_index: 3, src_index: 5, c: 0},
            Instruction {opcode: 0, dst_index: 2, src_index: 0, c: 0},
            Instruction {opcode: 3, dst_index: 0, src_index: 1, c: 0},
            Instruction {opcode: 1, dst_index: 0, src_index: 7, c: 4221005163},
            Instruction {opcode: 4, dst_index: 0, src_index: 2, c: 0},
            Instruction {opcode: 1, dst_index: 0, src_index: 7, c: 1789679560},
            Instruction {opcode: 5, dst_index: 0, src_index: 3, c: 0},
            Instruction {opcode: 1, dst_index: 2, src_index: 8, c: 2725270475},
            Instruction {opcode: 5, dst_index: 1, src_index: 4, c: 0},
            Instruction {opcode: 2, dst_index: 3, src_index: 8, c: 0},
            Instruction {opcode: 5, dst_index: 3, src_index: 5, c: 0},
            Instruction {opcode: 2, dst_index: 3, src_index: 2, c: 0},
            Instruction {opcode: 4, dst_index: 2, src_index: 2, c: 0},
            Instruction {opcode: 1, dst_index: 3, src_index: 6, c: 4110965463},
            Instruction {opcode: 5, dst_index: 2, src_index: 6, c: 0},
            Instruction {opcode: 2, dst_index: 2, src_index: 7, c: 0},
            Instruction {opcode: 2, dst_index: 3, src_index: 1, c: 0},
            Instruction {opcode: 2, dst_index: 1, src_index: 8, c: 0},
            Instruction {opcode: 3, dst_index: 1, src_index: 2, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 1, c: 0},
            Instruction {opcode: 0, dst_index: 2, src_index: 0, c: 0},
            Instruction {opcode: 6, dst_index: 0, src_index: 0, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 0, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 0, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 0, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 0, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 0, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 0, c: 0},
            Instruction {opcode: 0, dst_index: 0, src_index: 0, c: 0}
        ];

        for i in 0..9 {
            assert_eq!(r_expected[i], r[i], "r[{}] is incorrect", i);
        }
        for i in 0..71 {
            assert_eq!(code_expected[i], code[i], "code[{}] is incorrect", i);
        }
    }
}
