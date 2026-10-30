//! Pedersen hash
//!
//! This module handles executing the Pedersen hash algorithm over children in the Merkle tree.
use std::borrow::Cow;

use helioselene::{Helios, Selene, group::GroupEncoding};
use monero_fcmp_plus_plus_generators::{HELIOS_HASH_INIT, SELENE_HASH_INIT};
use rayon::prelude::*;

use crate::{
    Branch,
    traits::{Child, FcmpCurve},
};

/// An in-memory cache of an incomplete branch. This can be used to speed up calculation of the full
/// branch later and may represent the root of the tree, if the tree has incomplete children.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedIncompleteBranch<P: FcmpCurve, C: Child<P>> {
    /// The hash of the children currently present.
    pub branch: P::G,
    /// The potential incomplete child on the right of the lower layer.
    incomplete_child: Option<C>,
    /// The number of complete children in the chunk. There may be 1 more child the furthest right that is not counted
    /// if it is incomplete (`incomplete_child`).
    pub complete_children: usize,
}

/// A Pedersen hash computer that can compute chunks of hashes.
pub struct PedersenChunkHasher<P: FcmpCurve, C: Child<P>> {
    hash_init: P::G,
    cache: Option<CachedIncompleteBranch<P, C>>,
    state: PedersenChunkHasherState<C>,
}

/// The internal state of the [`PedersenChunkHasher`].
#[derive(Default)]
enum PedersenChunkHasherState<C> {
    /// Empty state.
    ///
    /// The hasher will compute the next hash from the first child given.
    #[default]
    Empty,
    /// Left over children from this hasher run.
    ///
    /// These children will be combined with the next given children to create a full chunk.
    LeftOver { left_over_children: Vec<C> },
}

impl<C: Child<Selene>> Default for PedersenChunkHasher<Selene, C> {
    fn default() -> Self {
        Self {
            hash_init: *SELENE_HASH_INIT,
            cache: None,
            state: PedersenChunkHasherState::Empty,
        }
    }
}

impl<C: Child<Helios>> Default for PedersenChunkHasher<Helios, C> {
    fn default() -> Self {
        Self {
            hash_init: *HELIOS_HASH_INIT,
            cache: None,
            state: PedersenChunkHasherState::Empty,
        }
    }
}

impl<C: Child<Selene>> From<CachedIncompleteBranch<Selene, C>> for PedersenChunkHasher<Selene, C> {
    fn from(value: CachedIncompleteBranch<Selene, C>) -> Self {
        Self {
            hash_init: *SELENE_HASH_INIT,
            cache: Some(value),
            state: PedersenChunkHasherState::Empty,
        }
    }
}

impl<C: Child<Helios>> From<CachedIncompleteBranch<Helios, C>> for PedersenChunkHasher<Helios, C> {
    fn from(value: CachedIncompleteBranch<Helios, C>) -> Self {
        Self {
            hash_init: *HELIOS_HASH_INIT,
            cache: Some(value),
            state: PedersenChunkHasherState::Empty,
        }
    }
}

impl<P: FcmpCurve, C: Child<P>> PedersenChunkHasher<P, C>
where
    [C]: ToOwned<Owned = Vec<C>>,
{
    /// Compute a chunk of full Pedersen hashes over the given children.
    ///
    /// All children passed in here must be full themselves if they are branches.
    ///
    /// Once you have passed in all children you can get the potential incomplete branch from [`PedersenChunkHasher::finish`].
    pub fn hash_children(&mut self, children: &[C]) -> Vec<Branch> {
        if children.is_empty() {
            return vec![];
        }

        let first_chunk_len =
            if let Some(full_children_cached) = self.cache.as_ref().map(|c| c.complete_children) {
                P::CHUNK_LEN - full_children_cached
            } else {
                P::CHUNK_LEN
            };

        // Get the first Pedersen hasher for the first chunk if we need to, then get the other
        // children which are not contained in that first hash.
        let (first_chunk, children) = match std::mem::take(&mut self.state) {
            PedersenChunkHasherState::Empty => {
                if children.len() < first_chunk_len {
                    self.state = PedersenChunkHasherState::LeftOver {
                        left_over_children: children.to_vec(),
                    };
                    return vec![];
                }

                let (l, r) = children.split_at(first_chunk_len);

                (Cow::Borrowed(l), r)
            }
            // Left over children from a previous call.
            PedersenChunkHasherState::LeftOver {
                mut left_over_children,
            } => {
                if left_over_children.len() + children.len() < first_chunk_len {
                    // If we still can't make a hash then just extend the leftover children and return now.
                    left_over_children.extend_from_slice(children);
                    self.state = PedersenChunkHasherState::LeftOver { left_over_children };
                    return vec![];
                }

                // We can make a hash, split the children used returning the first hash and the rest.
                let (first_chunk_needed, rest) =
                    children.split_at(first_chunk_len - left_over_children.len());

                left_over_children.extend_from_slice(first_chunk_needed);

                (Cow::Owned(left_over_children), rest)
            }
        };

        let first_chunk = if let Some(cache) = self.cache.take() {
            PedersenHasher::Cached {
                incomplete_branch: cache.branch,
                previous_complete_children: cache.complete_children,
                new_children: &first_chunk,
                incomplete_child: cache.incomplete_child,
            }
        } else {
            PedersenHasher::New {
                hash_init: self.hash_init,
                children: first_chunk,
            }
        };

        // Create the chunks iter over the rest of the children.
        let next_child_chunks = children.par_chunks_exact(P::CHUNK_LEN);
        // The children at the end that don't fit into a full chunk.
        let left_over_children = next_child_chunks.remainder();

        let full_chunks = rayon::iter::once(first_chunk)
            // Chain on the rest of the children we can hash.
            .chain(next_child_chunks.map(|child_chunk| PedersenHasher::New {
                hash_init: self.hash_init,
                children: Cow::Borrowed(child_chunk),
            }))
            // Do the hashing.
            .map_with(
                // Reusable scratch space to reduce allocations.
                Vec::with_capacity(P::CHUNK_LEN),
                |scratch_space, hash| hash.hash_with_scratch_space(scratch_space),
            )
            .map(|h| h.to_bytes())
            .collect::<Vec<_>>();

        // If we have children not hashed set the state to keep them.
        if !left_over_children.is_empty() {
            self.state = PedersenChunkHasherState::LeftOver {
                left_over_children: left_over_children.to_vec(),
            }
        }

        full_chunks
    }

    /// Finish the hasher, returning a [`CachedIncompleteBranch`] if there were leftover children.
    ///
    /// Accepts a potential incomplete child if there was an incomplete branch in the child layer.
    pub fn finish(
        &mut self,
        far_right_partial_child: Option<C>,
    ) -> Option<CachedIncompleteBranch<P, C>> {
        let (hash, complete_children) = match (
            std::mem::take(&mut self.state),
            far_right_partial_child.clone(),
        ) {
            // Empty state but partial child, the partial child is the start of a new branch.
            (PedersenChunkHasherState::Empty, Some(far_right_partial_child)) => {
                if let Some(cache) = self.cache.take() {
                    // No new children were consumed this run; the cached branch already
                    // contains the previous rightmost child. Re-hash only that position.
                    let hash = PedersenHasher::Cached {
                        incomplete_branch: cache.branch,
                        previous_complete_children: cache.complete_children,
                        new_children: std::slice::from_ref(&far_right_partial_child),
                        incomplete_child: cache.incomplete_child,
                    };

                    return Some(CachedIncompleteBranch {
                        branch: hash.hash_with_scratch_space(&mut Vec::new()),
                        complete_children: cache.complete_children,
                        incomplete_child: Some(far_right_partial_child),
                    });
                }

                let hash = PedersenHasher::New {
                    hash_init: self.hash_init,
                    children: Cow::Owned(vec![far_right_partial_child]),
                };

                (hash.hash_with_scratch_space(&mut Vec::new()), 0)
            }
            // Left over full children, maybe a partial child.
            (
                PedersenChunkHasherState::LeftOver {
                    mut left_over_children,
                },
                far_right_partial_child,
            ) => {
                let mut complete_children = left_over_children.len();
                // Add the partial child if Some.
                left_over_children.extend(far_right_partial_child);

                // Try to use the cache, we have to be careful enough children were given as someone could pass in less
                // children than the amount the passed in to make the cache in the previous instance.
                let hash = if let Some(cache) = self.cache.take() {
                    complete_children += cache.complete_children;

                    PedersenHasher::Cached {
                        incomplete_branch: cache.branch,
                        previous_complete_children: cache.complete_children,
                        new_children: &left_over_children,
                        incomplete_child: cache.incomplete_child,
                    }
                } else {
                    PedersenHasher::New {
                        hash_init: self.hash_init,
                        children: Cow::Owned(left_over_children),
                    }
                };

                (
                    hash.hash_with_scratch_space(&mut Vec::new()),
                    complete_children,
                )
            }
            // No other case will lead to a hash.
            _ => return self.cache.take(),
        };

        Some(CachedIncompleteBranch {
            branch: hash,
            complete_children,
            incomplete_child: far_right_partial_child,
        })
    }
}

/// A Pedersen hash computer.
///
/// This allows computation of a Pedersen hash over some given children.
enum PedersenHasher<'a, P: FcmpCurve, C: Child<P>> {
    /// Create a new hash.
    New {
        /// A fixed point to add to each Pedersen hash. `B_curve` in the FCMP paper.
        hash_init: P::G,
        /// The children to hash.
        children: Cow<'a, [C]>,
    },
    /// Update an already computed branch.
    Cached {
        /// The branch we want to update.
        incomplete_branch: P::G,
        /// The amount of _full_ children already hashed into the incomplete_branch;
        previous_complete_children: usize,
        /// The new children to hash, including the updated `incomplete_child` if there was one.
        new_children: &'a [C],
        /// The potential incomplete child, if the child layer also had an `incomplete_branch`.
        incomplete_child: Option<C>,
    },
}

impl<'a, P: FcmpCurve, C: Child<P>> PedersenHasher<'a, P, C>
where
    [C]: ToOwned<Owned = Vec<C>>,
{
    /// Compute the Pedersen hash with the given scratch space.
    ///
    /// The scratch space will be cleared before being used. Its contents after this function is
    /// unspecified, it is only used to reduce the number of allocations of multiple hashes are being
    /// computed.
    fn hash_with_scratch_space(&self, scratch_space: &mut Vec<(P::F, P::G)>) -> P::G {
        scratch_space.clear();

        match self {
            PedersenHasher::New {
                hash_init,
                children,
            } => {
                scratch_space.extend(
                    children
                        .iter()
                        .flat_map(|c| c.parent_scalars())
                        .zip(P::generators().generators.g_bold_slice())
                        .map(|(scalar, generator)| (scalar, *generator)),
                );

                *hash_init + multiexp::multiexp_vartime(scratch_space)
            }
            PedersenHasher::Cached {
                incomplete_branch,
                previous_complete_children,
                new_children,
                incomplete_child,
            } => {
                scratch_space.extend(
                    new_children
                        .iter()
                        .flat_map(|c| c.parent_scalars())
                        .zip(
                            P::generators()
                                .generators
                                .g_bold_slice()
                                .iter()
                                // Skip the generators we have already used.
                                .skip(*previous_complete_children * C::PARENT_SCALARS),
                        )
                        .map(|(scalar, generator)| (scalar, *generator)),
                );

                if let Some(incomplete_child) = incomplete_child {
                    // Only branches can be incomplete.
                    assert_eq!(C::PARENT_SCALARS, 1);
                    // If we had an incomplete child then we need to take the old one away from the new
                    // child in that space so we don't double count the grandchildren in the hash.
                    let mut incomplete_child = incomplete_child.parent_scalars().into_iter();
                    scratch_space[0].0 -= incomplete_child.next().unwrap();
                    assert!(incomplete_child.next().is_none());
                }

                *incomplete_branch + multiexp::multiexp_vartime(scratch_space)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::fmt::Debug;

    use ciphersuite::{Ciphersuite, group::Group};
    use curve25519_dalek::EdwardsPoint;
    use helioselene::{
        Helios, Selene,
        group::{
            GroupEncoding,
            ff::{Field, PrimeField},
        },
    };
    use monero_fcmp_plus_plus_generators::{HELIOS_HASH_INIT, SELENE_HASH_INIT};
    use rand::RngCore;

    use cuprate_types::fcmp::{FcmpLeafAmount, FcmpLeafData};

    use super::{CachedIncompleteBranch, PedersenChunkHasher, PedersenHasher};
    use crate::{
        Branch,
        traits::{Child, FcmpCurve},
    };

    fn generate_scalars<P: FcmpCurve>(amt: usize) -> Vec<[u8; 32]> {
        (0..amt)
            .map(|_| <P as Ciphersuite>::F::random(&mut rand::rngs::OsRng).to_repr())
            .collect()
    }

    fn generate_leafs(amt: usize) -> Vec<FcmpLeafData> {
        (0..amt)
            .map(|_| FcmpLeafData {
                o: EdwardsPoint::random(&mut rand::rngs::OsRng).compress().0,
                c: FcmpLeafAmount::Clear(rand::rngs::OsRng.next_u64()),
            })
            .collect()
    }

    #[test]
    fn single_hash_consistent() {
        fn single_hash_consistent_inner<P: FcmpCurve, GC: Child<P::Other>>(
            child_hash_init: <P::Other as Ciphersuite>::G,
            hash_init: P::G,

            gen_grandchildren: fn(usize) -> Vec<GC>,
            gen_children: fn(usize) -> Vec<Branch>,
        ) {
            let grandchildren = gen_grandchildren(<P::Other as FcmpCurve>::CHUNK_LEN);

            let child_hasher = PedersenHasher::<P::Other, _>::New {
                hash_init: child_hash_init,
                children: Cow::Borrowed(&grandchildren[..grandchildren.len() - 1]),
            };

            let partial_child = child_hasher.hash_with_scratch_space(&mut Vec::new());

            let mut full_children = gen_children(P::CHUNK_LEN - 1);

            full_children.push(partial_child.to_bytes());

            let hasher = PedersenHasher::<P, _>::New {
                hash_init,
                children: Cow::Borrowed(&full_children[..]),
            };

            let incomplete_branch = hasher.hash_with_scratch_space(&mut Vec::new());

            let child_hasher = PedersenHasher::<P::Other, _>::Cached {
                incomplete_branch: partial_child,
                previous_complete_children: grandchildren.len() - 1,
                incomplete_child: None,
                new_children: &grandchildren[grandchildren.len() - 1..],
            };

            let full_child = child_hasher.hash_with_scratch_space(&mut Vec::new());

            full_children.pop();
            full_children.push(full_child.to_bytes());

            let hasher = PedersenHasher::<P, _>::Cached {
                incomplete_branch,
                previous_complete_children: full_children.len() - 1,
                incomplete_child: Some(partial_child.to_bytes()),
                new_children: &full_children[full_children.len() - 1..],
            };

            let full_branch = hasher.hash_with_scratch_space(&mut Vec::new());

            let full_child_hasher = PedersenHasher::<P::Other, _>::New {
                hash_init: child_hash_init,
                children: Cow::Borrowed(&grandchildren),
            };
            let full_child = full_child_hasher.hash_with_scratch_space(&mut Vec::new());
            full_children.pop();
            full_children.push(full_child.to_bytes());

            let hasher = PedersenHasher::<P, _>::New {
                hash_init,
                children: Cow::Borrowed(&full_children[..]),
            };

            assert_eq!(full_branch, hasher.hash_with_scratch_space(&mut Vec::new()));
        }

        single_hash_consistent_inner::<Selene, Branch>(
            *HELIOS_HASH_INIT,
            *SELENE_HASH_INIT,
            generate_scalars::<Helios>,
            generate_scalars::<Selene>,
        );
        single_hash_consistent_inner::<Helios, Branch>(
            *SELENE_HASH_INIT,
            *HELIOS_HASH_INIT,
            generate_scalars::<Selene>,
            generate_scalars::<Helios>,
        );
        single_hash_consistent_inner::<Helios, FcmpLeafData>(
            *SELENE_HASH_INIT,
            *HELIOS_HASH_INIT,
            generate_leafs,
            generate_scalars::<Helios>,
        );
    }

    #[test]
    fn chunks_hasher_consistent() {
        fn chunks_hasher_consistent_inner<P: FcmpCurve, C: Child<P>>(
            gen_children: fn(usize) -> Vec<C>,
        ) where
            PedersenChunkHasher<P, C>: Default + From<CachedIncompleteBranch<P, C>>,
            CachedIncompleteBranch<P, C>: Debug + PartialEq,
        {
            let children = gen_children(10_000);

            let mut chunks_hasher = PedersenChunkHasher::<P, C>::default();
            let mut res = Vec::new();

            let mut i = 0;
            while i != children.len() {
                let r = rand::rngs::OsRng.next_u64() as usize % (children.len() - i) + 1;
                res.extend_from_slice(&chunks_hasher.hash_children(&children[i..(i + r)]));

                i += r;

                if rand::rngs::OsRng.next_u64().is_multiple_of(10) {
                    let cache = chunks_hasher.finish(None);
                    chunks_hasher = cache
                        .map(PedersenChunkHasher::from)
                        .unwrap_or_else(PedersenChunkHasher::default);
                }
            }

            let incomplete_branch = chunks_hasher.finish(None);
            let mut chunks_hasher = PedersenChunkHasher::<P, _>::default();

            let res2 = chunks_hasher.hash_children(&children);
            let incomplete_branch2 = chunks_hasher.finish(None);

            assert_eq!(incomplete_branch, incomplete_branch2);
            assert_eq!(res, res2);
        }

        chunks_hasher_consistent_inner::<Selene, Branch>(generate_scalars::<Selene>);
        chunks_hasher_consistent_inner::<Helios, Branch>(generate_scalars::<Helios>);
        chunks_hasher_consistent_inner::<Selene, FcmpLeafData>(generate_leafs);
    }
}
