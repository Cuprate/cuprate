use cuprate_cryptonight::{cryptonight_hash_r, cryptonight_hash_v0, cryptonight_hash_v1};
use rand::Rng;

fn main() {
    const COUNT: usize = 15;

    for _ in 0..COUNT {
        let mut rng = rand::thread_rng();
        let length = rng.gen_range(43..=500);
        let mut input = vec![0u8; length];
        rng.fill(&mut input[..]);

        let _ = cryptonight_hash_v0(&input);
        let _ = cryptonight_hash_v1(&input).unwrap();
        let _ = cryptonight_hash_r(&input, rng.random::<u64>());
    }
}
