use std::slice;

use cuprate_types::network::Network;

/// The hashes of the compiled in fast sync file.
///
/// See `build.rs` for how this file is generated.
static FAST_SYNC_HASHES: &[[u8; 32]] = &include!(concat!(env!("OUT_DIR"), "/fast_sync_hashes.rs"));

/// Set the fast-sync hashes according to the provided values.
pub fn set_fast_sync_hashes(fast_sync: bool, network: Network) {
    cuprate_fast_sync::set_fast_sync_hashes(if fast_sync && network == Network::Mainnet {
        FAST_SYNC_HASHES
    } else {
        &[]
    });
}

#[cfg(test)]
mod test {
    use super::*;

    /// Sanity check the fast sync hashes array.
    #[test]
    fn length() {
        assert!(FAST_SYNC_HASHES.len() > 6642);
    }
}
