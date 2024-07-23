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

fn cn_slow_hash(data: &[u8], variant: Variant) -> ([u8; 32], [u8; 200], u64, u64) {
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

    (
        b,
        state.get_keccak_state_bytes().clone(),
        division_result,
        sqrt_result,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum() {
        let input = hex::decode("5468697320697320612074657374205468697320697320612074657374205468697320697320612074657374").unwrap();
        let (b, hash_state, division_result, sqrt_result) = cn_slow_hash(&input, Variant::V2);

        assert_eq!(
            hex::encode(b),
            "00000000000000000000000000000000d99c9a4bae0badfb8a8cf8504b813b7d"
        );
        assert_eq!(hex::encode(hash_state), "af6fe96f8cb409bdd2a61fb837e346f1a28007b0f078a8d68bc1224b6fcfcc3c39f1244db8c0af06e94173db4a54038a2f7a6a9c729928b5ec79668a30cbf5f266110665e23e891ea4ee2337fb304b35bf8d9c2e4c3524e52e62db67b0b170487a68a34f8026a81b35dc835c60b356d2c411ad227b6c67e30e9b57ba34b3cf27fccecae972850cf3889bb3ff8347b55a5710d58086973d12d75a3340a39430b65ee2f4be27c21e7b39f47341dd036fe13bf43bb2c55bce498a3adcbf07397ea66062b66d56cd8136");
        assert_eq!(division_result, 1992885167645223034);
        assert_eq!(sqrt_result, 15156498822412360757);
    }
}
