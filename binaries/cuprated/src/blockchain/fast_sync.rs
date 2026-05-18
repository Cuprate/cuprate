use std::slice;

use cuprate_helper::network::Network;

/// The hashes of the compiled in fast sync file.
///
/// See `build.rs` for how this file is generated.
static FAST_SYNC_HASHES: &[[u8; 32]] = &include!(concat!(env!("OUT_DIR"), "/fast_sync_hashes.rs"));

/// Returns the fast-sync hashes for the given configuration.
///
/// Returns a non-empty slice only for mainnet with fast sync enabled.
pub fn get_fast_sync_hashes(fast_sync: bool, network: Network) -> &'static [[u8; 32]] {
    if fast_sync && network == Network::Mainnet {
        FAST_SYNC_HASHES
    } else {
        &[]
    }
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
