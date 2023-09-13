mod blake2_generator;
mod config;
//mod dataset;
mod registers;
mod superscalar;

fn is_0_or_power_of_2(x: u64) -> bool {
    (x & (x - 1)) == 0
}
