//! # Tree Builder
//!
//! This file contains the [`TreeBuilder`], this struct does the leaf and layer hashing required
//! to get the root of the tree.
use std::io;

use ciphersuite::group::GroupEncoding;
use helioselene::{Helios, HeliosPoint, Selene, SelenePoint};

use cuprate_types::fcmp::FcmpLeafData;

use crate::{
    Branch,
    pedersen_hash::{CachedIncompleteBranch, PedersenChunkHasher},
    traits::{Child, ChildReader, FcmpCurve, LayerWriter, NewLeaves, TreeStore},
};

/// The root of the FCMP tree.
#[derive(Clone)]
pub enum FcmpRoot {
    Helios(HeliosPoint),
    Selene(SelenePoint),
}

impl FcmpRoot {
    pub fn to_bytes(&self) -> [u8; 32] {
        match self {
            FcmpRoot::Helios(hp) => hp.to_bytes(),
            FcmpRoot::Selene(se) => se.to_bytes(),
        }
    }
}

/// Cached root data.
///
/// This contains the FCMP root and data to speed up adding more leaves to the tree.
pub struct CachedFcmpTreeRootData {
    /// The FCMP root, will be [`None`] if the tree is empty.
    pub root: Option<FcmpRoot>,
    cached_branches: CachedBranches,
}

/// Cached branches to speed up adding new leaves to the tree.
#[derive(Default)]
struct CachedBranches {
    /// The cached first layer.
    layer1: Option<CachedIncompleteBranch<Selene, FcmpLeafData>>,
    /// The cached helios layers, from bottom to top.
    helios: Vec<Option<CachedIncompleteBranch<Helios, Branch>>>,
    /// The cached selene layers, from bottom to top.
    selene: Vec<Option<CachedIncompleteBranch<Selene, Branch>>>,
}

impl CachedBranches {
    fn get_helios_layer(&self, layer_idx: usize) -> Option<CachedIncompleteBranch<Helios, Branch>> {
        let helios_idx = layer_idx >> 1;
        self.helios.get(helios_idx).cloned().flatten()
    }

    fn get_selene_layer(&self, layer_idx: usize) -> Option<CachedIncompleteBranch<Selene, Branch>> {
        assert_ne!(layer_idx, 0);

        let selene_idx = (layer_idx >> 1) - 1; // First layer is stored in its own field.
        self.selene.get(selene_idx).cloned().flatten()
    }

    fn push_helios(&mut self, new: Option<CachedIncompleteBranch<Helios, Branch>>) {
        self.helios.push(new);
    }

    fn push_selene(&mut self, new: Option<CachedIncompleteBranch<Selene, Branch>>) {
        self.selene.push(new)
    }

    fn try_get_root(&mut self, layer_idx: usize) -> Option<FcmpRoot> {
        if layer_idx == 0 {
            self.layer1.as_ref().map(|b| FcmpRoot::Selene(b.branch))
        } else if layer_idx & 1 == 1 {
            self.get_helios_layer(layer_idx)
                .map(|b| FcmpRoot::Helios(b.branch))
        } else {
            self.get_selene_layer(layer_idx)
                .map(|b| FcmpRoot::Selene(b.branch))
        }
    }
}

impl CachedFcmpTreeRootData {
    /// Create an empty [`CachedFcmpTreeRootData`].
    ///
    /// This can be passed into [`TreeBuilder::new`] and the same root will be generated as long as
    /// [`CachedFcmpTreeRootData::next_leaf_idx_needed`] is respected.
    pub fn empty() -> Self {
        Self {
            root: None,
            cached_branches: CachedBranches::default(),
        }
    }

    /// Returns the next leaf needed to extend the tree.
    pub fn next_leaf_idx_needed<T: TreeStore>(&self, tree_store: &T) -> Result<u64, io::Error> {
        Ok(
            tree_store.layer_writer(0).branch_len()? * Selene::CHUNK_LEN as u64
                + self
                    .cached_branches
                    .layer1
                    .as_ref()
                    .map_or(0, |c| c.complete_children as u64),
        )
    }
}

/// The tree builder.
pub struct TreeBuilder<T: TreeStore> {
    /// The tx for the lowest layer, exposed so you can add state to the backing store while building.
    pub layer_1_writer: T::LayerWriter,

    cached_branches: CachedBranches,
    tree_store: T,
    overflow: Option<Branch>,
    layer_1_len: u64,
}

impl<T: TreeStore> TreeBuilder<T> {
    /// Create a new tree builder to extend a tree.
    pub fn new(tree_store: T, old_root_data: CachedFcmpTreeRootData) -> Self {
        Self {
            layer_1_writer: tree_store.layer_writer(0),
            tree_store,
            cached_branches: old_root_data.cached_branches,
            overflow: None,
            layer_1_len: 0,
        }
    }

    /// Extend the tree with the given [`NewLeaves`]. The [`NewLeaves`] should start at the index provided
    /// by the [`CachedFcmpTreeRootData::next_leaf_idx_needed`] that was passed into [`TreeBuilder::new`].
    pub fn extend(&mut self, new_leaves: NewLeaves) -> Result<(), io::Error> {
        let (cached_incomplete_branch, layer_len1) = build_layer(
            &mut self.layer_1_writer,
            new_leaves,
            self.cached_branches.layer1.clone(),
            None,
        )?;
        self.layer_1_len = layer_len1;

        self.overflow = cached_incomplete_branch
            .as_ref()
            .map(|b| b.branch.to_bytes());

        self.cached_branches.layer1 = cached_incomplete_branch;

        Ok(())
    }

    /// Fills the upper layers of the tree returning the tree root with cached data.
    pub fn complete_tree(mut self) -> Result<CachedFcmpTreeRootData, io::Error> {
        self.layer_1_writer.commit()?;

        let mut new_cached_branches = CachedBranches {
            layer1: self.cached_branches.layer1.clone(),
            ..Default::default()
        };

        let mut layer_len = self.layer_1_len + u64::from(self.overflow.is_some());
        let mut layer_idx = 0;

        loop {
            if layer_len <= 1 {
                // We have a root!
                break;
            }

            layer_idx += 1;

            let mut layer_store = self.tree_store.layer_writer(layer_idx);
            let child_reader = self.tree_store.layer_reader(layer_idx - 1);

            match layer_idx {
                // Helios
                x if x & 1 == 1 => {
                    let cached_branch = self.cached_branches.get_helios_layer(layer_idx);

                    let (cached_incomplete_branch, layer_len2) =
                        build_layer(&mut layer_store, child_reader, cached_branch, self.overflow)?;
                    layer_len = layer_len2 + u64::from(cached_incomplete_branch.is_some());

                    self.overflow = cached_incomplete_branch
                        .as_ref()
                        .map(|b| b.branch.to_bytes());

                    new_cached_branches.push_helios(cached_incomplete_branch);
                }
                // Selene
                _ => {
                    let cached_branch = self.cached_branches.get_selene_layer(layer_idx);

                    let (cached_incomplete_branch, layer_len2) =
                        build_layer(&mut layer_store, child_reader, cached_branch, self.overflow)?;
                    layer_len = layer_len2 + u64::from(cached_incomplete_branch.is_some());

                    self.overflow = cached_incomplete_branch
                        .as_ref()
                        .map(|b| b.branch.to_bytes());

                    new_cached_branches.push_selene(cached_incomplete_branch);
                }
            }

            layer_store.commit()?;
        }

        match new_cached_branches.try_get_root(layer_idx) {
            // Root is in cache
            Some(root) => Ok(CachedFcmpTreeRootData {
                root: Some(root),
                cached_branches: new_cached_branches,
            }),
            // Tree is balanced or empty
            None => {
                let layer_store = self.tree_store.layer_reader(layer_idx);
                let root = layer_store.get(0)?;

                let root = root.map(|r| {
                    if layer_idx & 1 == 1 {
                        FcmpRoot::Helios(
                            HeliosPoint::from_bytes(&r).expect("Tapes database stored wrong point"),
                        )
                    } else {
                        FcmpRoot::Selene(
                            SelenePoint::from_bytes(&r).expect("Tapes database stored wrong point"),
                        )
                    }
                });

                Ok(CachedFcmpTreeRootData {
                    root,
                    cached_branches: new_cached_branches,
                })
            }
        }
    }
}

/// The layer builder.
///
/// This function extends the layer to the right for any missing children.
fn build_layer<P: FcmpCurve, C: Child<P> + Send + 'static, W: LayerWriter, R: ChildReader<C>>(
    layer_tx: &mut W,
    mut reader: R,
    cached_incomplete_branch: Option<CachedIncompleteBranch<P, C>>,
    prev_layer_overflow: Option<C>,
) -> Result<(Option<CachedIncompleteBranch<P, C>>, u64), io::Error>
where
    PedersenChunkHasher<P, C>: From<CachedIncompleteBranch<P, C>> + Default,
{
    let layer_len = layer_tx.branch_len()?;

    // start_index into the child layer
    let mut start_index = layer_len * P::CHUNK_LEN as u64
        + cached_incomplete_branch
            .as_ref()
            .map_or(0, |c| c.complete_children as u64);

    let mut chunk_hasher: PedersenChunkHasher<_, _> = cached_incomplete_branch
        .map(PedersenChunkHasher::from)
        .unwrap_or_default();

    loop {
        // The end index, either the size of the buffer of the end of the child layer.
        let children = reader.get_batch(start_index, P::CHUNK_LEN as u64 * 10_000)?;

        if children.is_empty() {
            break;
        }

        start_index += children.len() as u64;

        // Hash the children and return the hashes to write to our layer.
        let grow = chunk_hasher.hash_children(children.as_slice());

        // Write the hashes.
        layer_tx.append_branches(&grow)?;
    }

    let full_branches = layer_tx.branch_len()?;

    Ok((chunk_hasher.finish(prev_layer_overflow), full_branches))
}
