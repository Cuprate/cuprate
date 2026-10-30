//! Test that the tree is consistent after a reorg, across restarts and at edge sizes.
use std::{io, path::Path, sync::Arc};

use ciphersuite::group::Group;
use curve25519_dalek::{EdwardsPoint, constants::EIGHT_TORSION};
use rand::Rng;
use rand::rngs::OsRng;
use tempfile::TempDir;

use cuprate_fcmp_tree::{
    CachedFcmpTreeRootData, FcmpTreeStorageConfig, NewLeaves, TapeBlockInfo, TapesTreeStore,
    TreeBuilder,
};
use cuprate_types::fcmp::{FcmpLeafAmount, FcmpLeafData};

// With 1-4 leaves per block (~250 total on average) this gives committed branches in both the
// selene and helios layers.
const NUM_BLOCKS: usize = 100;
const MAX_LEAVES_PER_BLOCK: usize = 4;

/// Deterministic block hash, so blocks don't need to be stored.
fn block_hash(height: u64) -> [u8; 32] {
    let mut h = [0_u8; 32];
    h[..8].copy_from_slice(&height.to_le_bytes());
    h
}

fn open_store(dir: &Path) -> io::Result<TapesTreeStore> {
    TapesTreeStore::open(FcmpTreeStorageConfig {
        path: dir.to_path_buf(),
        layer_cache_size: 1_000,
    })
}

/// Cumulative leaf offsets for `num_blocks` blocks with 1-`max_leaves_per_block` leaves each.
fn random_leaf_offsets(
    rng: &mut OsRng,
    num_blocks: usize,
    max_leaves_per_block: usize,
) -> Vec<usize> {
    let mut offsets = vec![0usize; num_blocks + 1];
    for h in 0..num_blocks {
        offsets[h + 1] = offsets[h] + rng.gen_range(1..=max_leaves_per_block);
    }
    offsets
}

/// Random leaves. Every 7th leaf uses a torsion point and half the amounts are commitments, so
/// both branches of `FcmpLeafData::parent_scalars` get exercised.
fn random_leaves(rng: &mut OsRng, n: usize) -> Vec<FcmpLeafData> {
    // Added to a random point, this gives a leaf with a non-trivial torsion part, exercising
    // `clear_torsion`'s torsion branch (a pure torsion point would clear to the identity).
    let torsion = EIGHT_TORSION[1];

    let mut leaves = Vec::with_capacity(n);
    for i in 0..n {
        leaves.push(FcmpLeafData {
            o: if i % 7 == 3 {
                (EdwardsPoint::random(&mut *rng) + torsion).compress().0
            } else {
                EdwardsPoint::random(&mut *rng).compress().0
            },
            c: if rng.gen_bool(0.5) {
                FcmpLeafAmount::Clear(rng.gen_range(u64::MIN..u64::MAX))
            } else {
                FcmpLeafAmount::Commitment(EdwardsPoint::random(&mut *rng).compress().0)
            },
        });
    }
    leaves
}

/// Extend the tree to `target` height.
///
/// `leaf_offsets[h]` is the total number of leaves in blocks `0..h`. After a mid-chunk restart
/// these differ: leaves are re-hashed from the last complete chunk (`leaf_start`, see
/// [`CachedFcmpTreeRootData::next_leaf_idx_needed`]), while block infos are already committed for
/// every kept block (`block_start`).
fn extend_range(
    store: Arc<TapesTreeStore>,
    root_data: CachedFcmpTreeRootData,
    all_leaves: &[FcmpLeafData],
    leaf_offsets: &[usize],
    leaf_start: usize,
    block_start: usize,
    target: usize,
) -> Result<CachedFcmpTreeRootData, io::Error> {
    let mut builder = TreeBuilder::new(store.clone(), root_data);

    // Add the blocks to the store.
    builder.layer_1_writer.add_blocks(
        (block_start..target)
            .map(|h| TapeBlockInfo {
                hash: block_hash(h as u64),
                leaves_in_tree: leaf_offsets[h + 1] as u64,
            })
            .collect(),
    )?;

    // Extend the tree with the new leaves.
    builder.extend(NewLeaves {
        leaves: all_leaves[leaf_start..leaf_offsets[target]].to_vec(),
    })?;

    // Complete the tree and return the root data.
    builder.complete_tree()
}

/// Build a reference tree straight through, then build the same tree with random pops and
/// restarts in a second store, and check they agree.
fn run_consistency(num_blocks: usize, max_leaves_per_block: usize) -> io::Result<()> {
    let mut rng = OsRng;

    let leaf_offsets = random_leaf_offsets(&mut rng, num_blocks, max_leaves_per_block);
    let all_leaves = random_leaves(&mut rng, leaf_offsets[num_blocks]);

    // Two stores with the same data.
    let dir_a = TempDir::new()?;
    let dir_b = TempDir::new()?;
    let store_a = Arc::new(open_store(dir_a.path())?);
    let store_b = Arc::new(open_store(dir_b.path())?);

    // Reference tree: extend with everything, no pops.
    let root_a = extend_range(
        store_a.clone(),
        CachedFcmpTreeRootData::empty(),
        &all_leaves,
        &leaf_offsets,
        0,
        0,
        num_blocks,
    )?;

    // Test tree: randomly pop and add back while extending a random amount at a time.
    let mut height = 0usize;
    let mut leaf_start = 0usize;
    let mut root_b = CachedFcmpTreeRootData::empty();
    while height < num_blocks {
        let old_height = height;

        if rng.gen_bool(0.5) && height > 1 {
            // Pop a random amount of blocks, always leaving at least one.
            let pop_amt = rng.gen_range(1..height);
            store_b.pop_blocks((height - pop_amt) as u64)?;
            height -= pop_amt;

            // Restart from the committed tape, which may be mid-chunk and thus behind `height`.
            root_b = CachedFcmpTreeRootData::empty();
            leaf_start = root_b.next_leaf_idx_needed(&store_b)? as usize;
        }

        // Extend to a new high-water mark: always at least one block past the previous tip,
        // so the loop is guaranteed to make progress.
        let amt = std::cmp::min(
            std::cmp::max(rng.gen_range(1..=10), old_height + 1 - height),
            num_blocks - height,
        );

        // Block infos are only needed for the new blocks; leaves may need re-hashing from an
        // earlier point if we restarted mid-chunk.
        root_b = extend_range(
            store_b.clone(),
            root_b,
            &all_leaves,
            &leaf_offsets,
            leaf_start,
            height,
            height + amt,
        )?;
        height += amt;
        leaf_start = leaf_offsets[height];
    }

    assert_eq!(
        root_a.root.as_ref().map(|r| r.to_bytes()),
        root_b.root.as_ref().map(|r| r.to_bytes())
    );

    let (height_a, top_a) = store_a.top_block_in_tree()?.unwrap();
    let (height_b, top_b) = store_b.top_block_in_tree()?.unwrap();
    assert_eq!(height_a, height_b);
    assert_eq!(top_a.hash, top_b.hash);
    assert_eq!(top_a.leaves_in_tree, top_b.leaves_in_tree);

    // Check that every block is in the tree (and not under a wrong hash).
    for i in 0..num_blocks as u64 {
        let hash = block_hash(i);
        assert!(store_a.block_in_tree(i, hash)?);
        assert!(!store_a.block_in_tree(i, [0xFF; 32])?);
        assert!(store_b.block_in_tree(i, hash)?);
        assert!(!store_b.block_in_tree(i, [0xFF; 32])?);
    }

    Ok(())
}

#[test]
fn test_tree_consistent_after_reorg() -> io::Result<()> {
    run_consistency(NUM_BLOCKS, MAX_LEAVES_PER_BLOCK)
}

/// More leaves per block gives committed branches in three or more layers.
#[test]
fn test_tree_deep() -> io::Result<()> {
    run_consistency(NUM_BLOCKS, 64)
}

/// Boundary sizes: single leaf, partial chunks, and exact selene/helios chunk boundaries.
#[test]
fn test_tree_edge_sizes() -> io::Result<()> {
    for (blocks, max_leaves) in [(1, 1), (2, 1), (37, 1), (38, 1), (39, 1), (76, 1), (77, 1)] {
        run_consistency(blocks, max_leaves)?;
    }
    Ok(())
}

/// A store reopened from disk mid-chunk must be able to finish the tree and match a reference.
#[test]
fn test_tree_persistence_across_reopen() -> io::Result<()> {
    let mut rng = OsRng;

    let leaf_offsets = random_leaf_offsets(&mut rng, NUM_BLOCKS, MAX_LEAVES_PER_BLOCK);
    let all_leaves = random_leaves(&mut rng, leaf_offsets[NUM_BLOCKS]);

    // Reference tree: extend with everything, no pops.
    let dir_a = TempDir::new()?;
    let store_a = Arc::new(open_store(dir_a.path())?);
    let root_a = extend_range(
        store_a.clone(),
        CachedFcmpTreeRootData::empty(),
        &all_leaves,
        &leaf_offsets,
        0,
        0,
        NUM_BLOCKS,
    )?;

    // Build partway to a mid-chunk height, then drop the store (the directory stays).
    let mid = (1..NUM_BLOCKS)
        .find(|&h| !leaf_offsets[h].is_multiple_of(38))
        .unwrap();
    let dir_b = TempDir::new()?;
    {
        let store_b = Arc::new(open_store(dir_b.path())?);
        extend_range(
            store_b.clone(),
            CachedFcmpTreeRootData::empty(),
            &all_leaves,
            &leaf_offsets,
            0,
            0,
            mid,
        )?;
    }

    // Reopen from disk and finish. The committed tape is behind the tip, so leaves must be
    // re-hashed from the last complete chunk while block infos start at `mid`.
    let store_b = Arc::new(open_store(dir_b.path())?);
    let root_data = CachedFcmpTreeRootData::empty();
    let leaf_start = root_data.next_leaf_idx_needed(&store_b)? as usize;
    assert!(leaf_start < leaf_offsets[mid]);
    let root_b = extend_range(
        store_b.clone(),
        root_data,
        &all_leaves,
        &leaf_offsets,
        leaf_start,
        mid,
        NUM_BLOCKS,
    )?;

    assert_eq!(
        root_a.root.as_ref().map(|r| r.to_bytes()),
        root_b.root.as_ref().map(|r| r.to_bytes())
    );

    Ok(())
}
