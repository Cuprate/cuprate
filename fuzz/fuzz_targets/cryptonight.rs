#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};

fuzz_target!(|data: &[u8]| -> Corpus {
    if data.is_empty() {
        return Corpus::Reject;
    }

    match data[0] % 4 {
        0 => {
            cuprate_cryptonight::cryptonight_hash_v0(&data[1..]);
        }
        1 => {
            let _ = cuprate_cryptonight::cryptonight_hash_v1(&data[1..]);
        }
        2 => {
            cuprate_cryptonight::cryptonight_hash_v2(&data[1..]);
        }
        _ => {
            if data.len() < 9 {
                return Corpus::Reject;
            }

            cuprate_cryptonight::cryptonight_hash_r(
                &data[9..],
                u64::from_le_bytes(data[1..9].try_into().unwrap()),
            );
        }
    }

    Corpus::Keep
});
