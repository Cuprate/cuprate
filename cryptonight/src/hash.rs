use std::io::Write;

use keccak;
use sha3::Digest;

struct cache {
    scratchpad: [u64; 2 * 1024 * 1024 / 8], // 2 MiB scratchpad
    final_state: [u64; 25],                 // state of keccak1600
    _padding: [u8; 8],                       // ensure that next field is 16 byte aligned
    blocks: [u64; 16],                      // temporary chunk/pointer of data
    rkeys: [u32; 40],                       // 10 rounds, instead of 14 as in standard AES-256
}

impl Default for cache {
    fn default() -> Self {
        cache {
            scratchpad: [0; 2 * 1024 * 1024 / 8],
            final_state: [0; 25],
            _padding: [0; 8],
            blocks: [0; 16],
            rkeys: [0; 40],
        }
    }
}

impl cache {
    // fn zero(&mut self) {
    //     *self = cache::default();
    // }
}


// pub fn convert(data: &[u32; 4]) -> [u8; 16] {
//     unsafe { std::mem::transmute(*data) }
// }

const PLEN: usize = 25;
const TLEN: usize = 144;


#[inline(always)]
#[allow(cast_ptr_alignment)]
fn as_u64_array(t: &mut [u8; TLEN]) -> &mut [u64; TLEN / 8] {
    unsafe { &mut *(t as *mut [u8; TLEN] as *mut [u64; TLEN / 8]) }
}

pub fn as_u8_array(t: &mut [u64; 25]) -> &mut [u8; 200] {
    unsafe { &mut *(t as *mut [u64; 25] as *mut [u8; 200]) }
}

fn xorin(dst: &mut [u8], src: &[u8]) {
    for (d, i) in dst.iter_mut().zip(src) {
        *d ^= *i;
    }
}


pub fn cn_keccak(input: &[u8]) -> [u64; 25] { // [u8; 200] {

    let mut a: [u64; PLEN] = [0; PLEN];
    let init_rate = 136; //200 - 512/4;
    let mut rate = init_rate;
    let inlen = input.len();
    let mut tmp: [u8; TLEN] = [0; TLEN];
    tmp[..inlen].copy_from_slice(input);

    //first foldp
    let mut ip = 0;
    let mut l = inlen;
    while l >= rate {
        xorin(&mut as_u8_array(&mut a)[0..][..rate], &input[ip..]);
        tiny_keccak::keccakf(&mut a);
        ip += rate;
        l -= rate;
        rate = init_rate;
    }

    //pad
    tmp[inlen] = 1;
    tmp[rate - 1] |= 0x80;

    let t64 = as_u64_array(&mut tmp);
    for i in 0..(rate / 8) {
        a[i] ^= t64[i];
    }

    keccak::f1600(&mut a);

    //let t8 = as_u8_array(&mut a);
    a
}


fn sum(data: &[u8]) -> [u64; 25] { //-> [u8; 32] {
    //let mut cache = cache::default();

    // let mut a: [u64; 2] = [0; 2];
    // let mut b: [u64; 2] = [0; 2];
    // let mut c: [u64; 2] = [0; 2];
    // let mut d: [u64; 2] = [0; 2];
    // let mut v1_tweak: u64 = 0;
    // let mut e: [u64; 2] = [0; 2];
    // let mut div_result: u64 = 0;
    // let mut sqrt_result: u64 = 0;

    //let mut hasher = sha3::Keccak256Full::new();
    // hasher.write(data).expect("Failed to write data to hasher");
    // let result = hasher.finalize().as_slice();
    // for i in 0..25 {
    //   cache.final_state[i] = u64::from_le_bytes(result[i*8..(i+1)*8-1]);
    // }

    cn_keccak(data)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum() {
        let input = hex::decode("6465206f6d6e69627573206475626974616e64756d").unwrap();
        let result = sum(&input);
        println!("{:?}", result);

        let expected: [u64; 25] = [65988738957872738, 2194301957348098446, 17365506734217090054, 6929014052009831719, 10758072901270498607, 17614083051140728683, 16912734431697773670, 17995738129103446497, 11186838957039213313, 1469340006088065277, 4004907566736267822, 1475774541647820153, 17123339490728040073, 10382314527516006478, 335215056686190860, 15195246702211564693, 7962146138469427773, 15060537934304135993, 16835885316047090052, 9338711154135663907, 18110028952740226285, 6308362351329590607, 9210355527720618686, 5800322365230761356, 5757769281071104184];
        assert_eq!(result, expected);
    }
}
