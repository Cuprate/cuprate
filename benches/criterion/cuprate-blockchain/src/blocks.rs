use rand::Rng;

use cuprate_helper::cast::u64_to_usize;
use cuprate_types::VerifiedBlockInformation;

/// Generate fake [`VerifiedBlockInformation`]s.
///
/// This function generates fake blocks,
/// cloning the `base` block `count` amount of times,
/// starting at height `0` and sequentially incrementing,
/// i.e. block height `{0,1,2,...}`.
///
/// Each block has a random [`VerifiedBlockInformation::block_hash`].
///
/// # Hack
/// This is used for these benchmarks because
/// [`cuprate_blockchain::ops::block::add_block`]
/// asserts that blocks with non-sequential heights cannot be added.
/// To get around this, we manually edit the block heights.
///
/// The hash must also be faked as
/// [`cuprate_blockchain::ops::blockchain::chain_height`]
/// which is used for an [`assert`] relies on the `hash -> height` table.
pub fn generate_fake_blocks(
    base: &VerifiedBlockInformation,
    count: u64,
) -> Vec<VerifiedBlockInformation> {
    (0..count)
        .map(|height| {
            let mut block = base.clone();
            block.height = u64_to_usize(height);
            block.block_hash = rand::thread_rng().r#gen();
            block
        })
        .collect()
}
