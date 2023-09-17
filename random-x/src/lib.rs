mod aes_hash;
mod blake2_generator;
mod config;
mod dataset;
mod registers;
mod superscalar;

pub use dataset::{Cache, Dataset};

fn is_0_or_power_of_2(x: u64) -> bool {
    (x & (x - 1)) == 0
}
