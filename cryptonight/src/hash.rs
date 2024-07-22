use std::io::Write;

use sha3::Digest;

struct Cache {
    scratchpad: [u64; 2 * 1024 * 1024 / 8], // 2 MiB scratchpad
    final_state: [u64; 25],                 // state of keccak1600
    _padding: [u8; 8],                       // ensure that next field is 16 byte aligned
    blocks: [u64; 16],                      // temporary chunk/pointer of data
    rkeys: [u32; 40],                       // 10 rounds, instead of 14 as in standard AES-256
}

impl Default for Cache {
    fn default() -> Self {
        Cache {
            scratchpad: [0; 2 * 1024 * 1024 / 8],
            final_state: [0; 25],
            _padding: [0; 8],
            blocks: [0; 16],
            rkeys: [0; 40],
        }
    }
}

impl Cache {
    // fn zero(&mut self) {
    //     *self = cache::default();
    // }
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

    let mut hasher = sha3::Keccak256Full::new();
    hasher.write(data).expect("Failed to write data to hasher");
    let hash = hasher.finalize().to_vec();
    assert_eq!(hash.len(), 200);

    let state: &mut [u64; 25] = &mut [0; 25];
    for i in 0..25 {
        state[i] = u64::from_le_bytes([hash[i * 8], hash[i * 8 + 1], hash[i * 8 + 2], hash[i * 8 + 3], hash[i * 8 + 4], hash[i * 8 + 5], hash[i * 8 + 6], hash[i * 8 + 7]]);
    }

    println!("state: {:?}", state);


    state.clone()
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
