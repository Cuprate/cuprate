use randomx_rs::{RandomXCache, RandomXDataset, RandomXFlag, RandomXVM};

/// Returns a [`RandomXVM`] with no optimization flags (default, light-verification).
pub fn randomx_vm_default(seed_hash: &[u8; 32]) -> RandomXVM {
    const FLAG: RandomXFlag = RandomXFlag::FLAG_DEFAULT;

    let cache = RandomXCache::new(FLAG, seed_hash).unwrap();
    RandomXVM::new(FLAG, Some(cache), None).unwrap()
}

/// Returns a [`RandomXVM`] with most optimization flags.
#[expect(dead_code)]
pub fn randomx_vm_optimized(seed_hash: &[u8; 32]) -> RandomXVM {
    // TODO: conditional FLAG_LARGE_PAGES, FLAG_JIT

    let vm_flag = RandomXFlag::get_recommended_flags() | RandomXFlag::FLAG_FULL_MEM;
    let cache_flag = RandomXFlag::get_recommended_flags();

    let hash = hex::encode(seed_hash);

    println!("Generating RandomX VM: seed_hash: {hash}, flags: {vm_flag:#?}");
    let cache = RandomXCache::new(cache_flag, seed_hash).unwrap();
    let dataset = RandomXDataset::new(RandomXFlag::FLAG_DEFAULT, cache, 0).unwrap();
    let vm = RandomXVM::new(vm_flag, None, Some(dataset)).unwrap();
    println!("Generating RandomX VM: seed_hash: {hash}, flags: {vm_flag:#?} ... OK");

    vm
}
