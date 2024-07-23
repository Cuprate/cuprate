use std::cmp::PartialEq;
use std::io::Write;

use memoffset::offset_of;
use sha3::Digest;
use static_assertions::const_assert_eq;

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

fn keccak1600(input: &[u8], out: &mut [u8; 200]) {
    let mut hasher = sha3::Keccak256Full::new();
    hasher.write(input).unwrap();
    let result = hasher.finalize();
    out.copy_from_slice(result.as_slice());
}

const NONCE_PTR_INDEX: usize = 35;

fn cn_slow_hash(data: &[u8], variant: Variant) -> ([u8; 8], [u8; 200]) {
    // -> [u8; 32] {
    //-> [u8; 32] {
    // let long_state: [u8; MEMORY];
    // let text: [u8; INIT_SIZE_BYTE];
    // let a: [u8; AES_BLOCK_SIZE];
    // let a1: [u8; AES_BLOCK_SIZE];
    // let b: [u8; AES_BLOCK_SIZE * 2];
    // let c1: [u8; AES_BLOCK_SIZE];
    // let c2: [u8; AES_BLOCK_SIZE];
    // let d: [u8; AES_BLOCK_SIZE];
    // let aes_key: [u8; AES_KEY_SIZE];

    let mut state = CnSlowHashState::default();
    let mut keccak_state_bytes = state.get_keccak_state_bytes_mut();
    keccak1600(data, &mut keccak_state_bytes);

    let mut tweak1_2 = [0u8; 8];
    if variant == Variant::V1 {
        assert!(
            data.len() >= 43,
            "Cryptonight variant 1 needs at least 43 bytes of data"
        );
        tweak1_2.copy_from_slice(keccak_state_bytes[192..200].iter().as_slice());
        xor64(&mut tweak1_2, &data[NONCE_PTR_INDEX..NONCE_PTR_INDEX + 8]);
    }

    (tweak1_2.clone(), state.get_keccak_state_bytes().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum() {
        let input = hex::decode("8519e039172b0d70e5ca7b3383d6b3167315a422747b73f019cf9528f0fde341fd0f2a63030ba6450525cf6de31837669af6f1df8131faf50aaab8d3a7405589").unwrap();
        let (tweak, hash_state) = cn_slow_hash(&input, Variant::V1);
        assert_eq!(hex::encode(tweak), "9d0192226fb7e413");
        assert_eq!(hex::encode(hash_state), "daedcec20429ffd440ab70a0a3a549fbc89745581f539fd2ac945388698e2db238bad5006189a23b7a520fe71706f121d8f8bbd70334ef5609ad9f8c332819363a522cca3d9aac50ff095e6f9b0f215a08ab179f472ecacc1446c281aa07fdddbed3441bb4d9284e846bab2bb0efea65423906bc338292e229656b1e5f994560f69d6eed5fc31ec8d1b565ae7ea4adfd18ade82774b3e2efac16bb052060306fe8411bad32867e3c7b7299edba6cc67fd05fe9323335dd7ba62cc870f16bf561fe0299842ab2c1dc");
    }
}
