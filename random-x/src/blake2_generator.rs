use blake2::digest::FixedOutputReset;
use blake2::{Blake2b512, Digest};

const MAX_SEED_LEN: usize = 60;

pub struct Blake2Generator {
    data: [u8; 64],
    index: usize,
    hasher: Blake2b512,
}

impl Blake2Generator {
    pub fn new(seed: &[u8], nonce: u32) -> Self {
        assert!(seed.len() <= MAX_SEED_LEN);

        let mut data = [0; 64];
        data[..seed.len()].copy_from_slice(seed);

        data[MAX_SEED_LEN..].copy_from_slice(&nonce.to_le_bytes());

        Blake2Generator {
            data,
            index: 64,
            hasher: Blake2b512::default(),
        }
    }

    pub fn next_u8(&mut self) -> u8 {
        self.check_extend(1);
        self.index += 1;
        self.data[self.index - 1]
    }

    pub fn next_u32(&mut self) -> u32 {
        self.check_extend(4);
        self.index += 4;
        u32::from_le_bytes(self.data[self.index - 4..self.index].try_into().unwrap())
    }

    fn check_extend(&mut self, bytes_needed: usize) {
        if self.index + bytes_needed > self.data.len() {
            self.hasher.update(self.data);
            self.data = self.hasher.finalize_fixed_reset().into();
            self.index = 0;
        }
    }
}
