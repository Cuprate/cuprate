use std::io::Write;

use memoffset::offset_of;
use sha3::digest::{DynDigest, FixedOutput};
use sha3::Digest;
use static_assertions::const_assert_eq;

const MEMORY: usize = 1 << 21; // 2MB scratchpad
const ITER: usize = 1 << 20;
const AES_BLOCK_SIZE: usize = 16;
const AES_KEY_SIZE: usize = 32;
const INIT_SIZE_BLK: usize = 8;
const INIT_SIZE_BYTE: usize = INIT_SIZE_BLK * AES_BLOCK_SIZE;

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

    fn get_mut_keccak_state_words(&mut self) -> &mut [u64; 25] {
        // SAFETY: Compile time alignment checks ensure that the union'ed fields
        // overlap correctly with no unexpected padding. This local-only type
        // is never shared between threads.
        unsafe { &mut self.hs.w }
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

fn keccak1600(input: &[u8], out: &mut [u8; 200]) {
    let mut hasher = sha3::Keccak256Full::new();
    hasher.write(input).unwrap();
    let result = hasher.finalize();
    out.copy_from_slice(result.as_slice());
}

fn cn_slow_hash(data: &[u8]) -> [u64; 25] {
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
    keccak1600(data, state.get_keccak_state_bytes_mut());
    state.get_mut_keccak_state_words().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum() {
        let input = hex::decode("6465206f6d6e69627573206475626974616e64756d").unwrap();
        let result = cn_slow_hash(&input);
        let expected: [u64; 25] = [
            65988738957872738,
            2194301957348098446,
            17365506734217090054,
            6929014052009831719,
            10758072901270498607,
            17614083051140728683,
            16912734431697773670,
            17995738129103446497,
            11186838957039213313,
            1469340006088065277,
            4004907566736267822,
            1475774541647820153,
            17123339490728040073,
            10382314527516006478,
            335215056686190860,
            15195246702211564693,
            7962146138469427773,
            15060537934304135993,
            16835885316047090052,
            9338711154135663907,
            18110028952740226285,
            6308362351329590607,
            9210355527720618686,
            5800322365230761356,
            5757769281071104184,
        ];
        assert_eq!(result, expected);
    }
}
