#![no_main]

use libfuzzer_sys::fuzz_target;

use monero_serai::transaction::{Transaction, NotPruned};

fuzz_target!(|data: &[u8]| {
    drop(Transaction::<NotPruned>::read(&mut &data[..]));
});