fn main() {
    generate_fast_sync_hashes();
}

/// Generates `fast_sync_hashes.rs` from `fast_sync_hashes.json`.
///
/// This creates a temporary build file with the
/// `Debug` representation of the hashes, i.e.:
/// ```
/// [[0, 1, 2, ...], [0, 1, 2, ...], [0, 1, 2, ...]]
/// ```
///
/// This is then used in `cuprated` with:
/// ```rust
/// let _: &[[u8; 32]] = &include!(...)
/// ```
fn generate_fast_sync_hashes() {
    println!("cargo::rerun-if-changed=src/blockchain/fast_sync/fast_sync_hashes.json");

    let hashes = serde_json::from_str::<Vec<cuprate_hex::Hex<32>>>(include_str!(
        "src/blockchain/fast_sync/fast_sync_hashes.json"
    ))
    .unwrap()
    .into_iter()
    .map(|h| h.0)
    .collect::<Vec<[u8; 32]>>();

    std::fs::write(
        format!("{}/fast_sync_hashes.rs", std::env::var("OUT_DIR").unwrap()),
        format!("{hashes:?}"),
    )
    .unwrap();
}
