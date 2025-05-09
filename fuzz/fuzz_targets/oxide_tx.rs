#![no_main]

use libfuzzer_sys::fuzz_target;

use monero_serai::transaction::{NotPruned, Transaction};

fuzz_target!(|data: &[u8]| {
    drop(Transaction::<NotPruned>::read(&mut &data[..]));
});
