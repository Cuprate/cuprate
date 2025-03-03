use std::slice;

use cuprate_helper::network::Network;

static FAST_SYNC_HASHES: &[[u8; 32]] = unsafe {
    let bytes = include_bytes!("./fast_sync/fast_sync_hashes.bin");
    if bytes.len() % 32 != 0 {
        panic!()
    }

    slice::from_raw_parts(bytes.as_ptr().cast::<[u8; 32]>(), bytes.len() / 32)
};

pub fn set_fast_sync_hashes(fast_sync: bool, network: Network) {
    cuprate_fast_sync::fast_sync::set_fast_sync_hashes(
        if fast_sync && network == Network::Mainnet {
            FAST_SYNC_HASHES
        } else {
            &[]
        },
    );
}
