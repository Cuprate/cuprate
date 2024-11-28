//! General constants.

use cuprate_blockchain::types::{Output, OutputFlags, PreRctOutputId};

/// The (1st) key.
pub const KEY: PreRctOutputId = PreRctOutputId {
    amount: 1,
    amount_index: 123,
};

/// The expected value.
pub const VALUE: Output = Output {
    key: [35; 32],
    height: 45_761_798,
    output_flags: OutputFlags::empty(),
    tx_idx: 2_353_487,
};
