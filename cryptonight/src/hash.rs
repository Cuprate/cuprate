struct Cache {
    scratchpad: [u64; 2 * 1024 * 1024 / 8], // 2 MiB scratchpad
    final_state: [u64; 25],                 // state of keccak1600
    _padding: [u8; 8],                       // ensure that next field is 16 byte aligned
    blocks: [u64; 16],                      // temporary chunk/pointer of data
    rkeys: [u32; 40],                       // 10 rounds, instead of 14 as in standard AES-256
}


fn Sum(data: &[u8]) -> [u8; 32] {
    let mut cache = Cache {
        scratchpad: [0; 2 * 1024 * 1024 / 8],
        final_state: [0; 25],
        _padding: [0; 8],
        blocks: [0; 16],
        rkeys: [0; 40],
    };

    let mut hash = [0; 32];
    unsafe {
        cn_slow_hash(data.as_ptr(), data.len(), hash.as_mut_ptr(), 0, 0, 0);
    }
    hash
}
