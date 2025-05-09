#![no_main]

use libfuzzer_sys::fuzz_target;

use monero_serai::block::Block;

fuzz_target!(|data: &[u8]| {
    drop(Block::read(&mut &data[..]));
});
