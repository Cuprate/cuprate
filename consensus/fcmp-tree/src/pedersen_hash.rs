//! Pedersen hash
//!
//! This module handles executing the Pedersen hash algorithm over children in the Merkle tree.
use std::borrow::Cow;

use helioselene::group::GroupEncoding;
use rayon::prelude::*;

use crate::{
    Branch,
    traits::{Child, FcmpCurve},
};

/// An in-memory cache of an incomplete branch. This can be used to speed up calculation of the full
/// branch later and may represent the root of the tree, if the tree has incomplete children.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CachedIncompleteBranch<P: FcmpCurve, C: Child<P>> {
    /// The hash of the children currently present.
    pub(crate) branch: P::G,
    /// The potential incomplete child on the right of the lower layer.
    incomplete_child: Option<C>,
    /// The number of complete children in the chunk. There may be 1 more child the furthest right that is not counted
    /// if it is incomplete (`incomplete_child`).
    pub(crate) complete_children: usize,
}

/// A Pedersen hash computer that can compute chunks of hashes.
pub(crate) struct PedersenChunkHasher<P: FcmpCurve, C: Child<P>> {
    cache: Option<CachedIncompleteBranch<P, C>>,
    /// Left over children from this hasher run.
    ///
    /// These children will be combined with the next given children to create a full chunk.
    left_over_children: Vec<C>,
}

impl<P: FcmpCurve, C: Child<P>> PedersenChunkHasher<P, C>
where
    [C]: ToOwned<Owned = Vec<C>>,
{
    /// Create a hasher that continues the cached incomplete branch, if there is one.
    pub(crate) const fn new(cache: Option<CachedIncompleteBranch<P, C>>) -> Self {
        Self {
            cache,
            left_over_children: Vec::new(),
        }
    }

    /// Compute a chunk of full Pedersen hashes over the given children.
    ///
    /// All children passed in here must be full themselves if they are branches.
    ///
    /// Once you have passed in all children you can get the potential incomplete branch from [`PedersenChunkHasher::finish`].
    pub(crate) fn hash_children(&mut self, children: &[C]) -> Vec<Branch> {
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
        if self.left_over_children.len() + children.len() < first_chunk_len {
            // If we still can't make a hash then just extend the leftover children and return now.
            self.left_over_children.extend_from_slice(children);
            return vec![];
        }

        // We can make a hash, split the children used returning the first hash and the rest.
        let (first_chunk_needed, children) =
            children.split_at(first_chunk_len - self.left_over_children.len());

        let first_chunk = if self.left_over_children.is_empty() {
            Cow::Borrowed(first_chunk_needed)
        } else {
            let mut left_over_children = std::mem::take(&mut self.left_over_children);
            left_over_children.extend_from_slice(first_chunk_needed);
            Cow::Owned(left_over_children)
        };

        let first_chunk = PedersenHasher::new(self.cache.take(), &first_chunk);

        // Create the chunks iter over the rest of the children.
        let next_child_chunks = children.par_chunks_exact(P::CHUNK_LEN);
        // The children at the end that don't fit into a full chunk.
        let left_over_children = next_child_chunks.remainder();

        let full_chunks = rayon::iter::once(first_chunk)
            // Chain on the rest of the children we can hash.
            .chain(next_child_chunks.map(|child_chunk| PedersenHasher::new(None, child_chunk)))
            // Do the hashing.
            .map_with(
                // Reusable scratch space to reduce allocations.
                Vec::with_capacity(P::CHUNK_LEN),
                |scratch_space, hash| hash.hash_with_scratch_space(scratch_space),
            )
            .map(|h| h.to_bytes())
            .collect::<Vec<_>>();

        // If we have children not hashed keep them.
        self.left_over_children = left_over_children.to_vec();

        full_chunks
    }

    /// Finish the hasher, returning a [`CachedIncompleteBranch`] if there were leftover children.
    ///
    /// Accepts a potential incomplete child if there was an incomplete branch in the child layer.
    pub(crate) fn finish(
        &mut self,
        far_right_partial_child: Option<C>,
    ) -> Option<CachedIncompleteBranch<P, C>> {
        let mut left_over_children = std::mem::take(&mut self.left_over_children);
        let mut complete_children = left_over_children.len();
        // Add the partial child if Some.
        left_over_children.extend(far_right_partial_child.clone());

        // Nothing to hash.
        if left_over_children.is_empty() {
            return self.cache.take();
        }

        // Try to use the cache, we have to be careful enough children were given as someone could pass in less
        // children than the amount the passed in to make the cache in the previous instance.
        if let Some(cache) = &self.cache {
            complete_children += cache.complete_children;
        }

        let hash = PedersenHasher::new(self.cache.take(), &left_over_children);

        Some(CachedIncompleteBranch {
            branch: hash.hash_with_scratch_space(&mut Vec::new()),
            complete_children,
            incomplete_child: far_right_partial_child,
        })
    }
}

/// A Pedersen hash computer.
///
/// This allows computation of a Pedersen hash over some given children.
struct PedersenHasher<'a, P: FcmpCurve, C: Child<P>> {
    /// The branch to add the hash onto, the fixed point `B_curve` in the FCMP paper for a new branch.
    base: P::G,
    /// The amount of _full_ children already hashed into `base`.
    offset: usize,
    /// The children to hash, including the updated `incomplete_child` if there was one.
    children: &'a [C],
    /// The potential incomplete child, if the child layer also had an `incomplete_branch`.
    incomplete_child: Option<C>,
}

impl<'a, P: FcmpCurve, C: Child<P>> PedersenHasher<'a, P, C>
where
    [C]: ToOwned<Owned = Vec<C>>,
{
    /// Create a hasher over the children, updating the cached incomplete branch if there is one.
    fn new(cache: Option<CachedIncompleteBranch<P, C>>, children: &'a [C]) -> Self {
        match cache {
            Some(cache) => Self {
                base: cache.branch,
                offset: cache.complete_children,
                children,
                incomplete_child: cache.incomplete_child,
            },
            None => Self {
                base: P::hash_init(),
                offset: 0,
                children,
                incomplete_child: None,
            },
        }
    }

    /// Compute the Pedersen hash with the given scratch space.
    ///
    /// The scratch space will be cleared before being used. Its contents after this function is
    /// unspecified, it is only used to reduce the number of allocations of multiple hashes are being
    /// computed.
    fn hash_with_scratch_space(&self, scratch_space: &mut Vec<(P::F, P::G)>) -> P::G {
        scratch_space.clear();

        scratch_space.extend(
            self.children
                .iter()
                .flat_map(Child::parent_scalars)
                .zip(
                    P::generators()
                        .generators
                        .g_bold_slice()
                        .iter()
                        // Skip the generators we have already used.
                        .skip(self.offset * C::PARENT_SCALARS),
                )
                .map(|(scalar, generator)| (scalar, *generator)),
        );

        if let Some(incomplete_child) = &self.incomplete_child {
            // Only branches can be incomplete.
            assert_eq!(C::PARENT_SCALARS, 1);
            // If we had an incomplete child then we need to take the old one away from the new
            // child in that space so we don't double count the grandchildren in the hash.
            let mut incomplete_child = incomplete_child.parent_scalars().into_iter();
            scratch_space[0].0 -= incomplete_child.next().unwrap();
            assert!(incomplete_child.next().is_none());
        }

        self.base + multiexp::multiexp_vartime(scratch_space)
    }
}

#[cfg(test)]
mod tests {
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
    use rand::RngCore;

    use cuprate_helper::cast::u64_to_usize;
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

    fn generate_leaves(amt: usize) -> Vec<FcmpLeafData> {
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
            gen_grandchildren: fn(usize) -> Vec<GC>,
            gen_children: fn(usize) -> Vec<Branch>,
        ) {
            let grandchildren = gen_grandchildren(<P::Other as FcmpCurve>::CHUNK_LEN);

            let child_hasher =
                PedersenHasher::<P::Other, _>::new(None, &grandchildren[..grandchildren.len() - 1]);

            let partial_child = child_hasher.hash_with_scratch_space(&mut Vec::new());

            let mut full_children = gen_children(P::CHUNK_LEN - 1);

            full_children.push(partial_child.to_bytes());

            let hasher = PedersenHasher::<P, _>::new(None, &full_children);

            let incomplete_branch = hasher.hash_with_scratch_space(&mut Vec::new());

            let child_hasher = PedersenHasher::<P::Other, _> {
                base: partial_child,
                offset: grandchildren.len() - 1,
                children: &grandchildren[grandchildren.len() - 1..],
                incomplete_child: None,
            };

            let full_child = child_hasher.hash_with_scratch_space(&mut Vec::new());

            full_children.pop();
            full_children.push(full_child.to_bytes());

            let hasher = PedersenHasher::<P, _> {
                base: incomplete_branch,
                offset: full_children.len() - 1,
                children: &full_children[full_children.len() - 1..],
                incomplete_child: Some(partial_child.to_bytes()),
            };

            let full_branch = hasher.hash_with_scratch_space(&mut Vec::new());

            let full_child_hasher = PedersenHasher::<P::Other, _>::new(None, &grandchildren);
            let full_child = full_child_hasher.hash_with_scratch_space(&mut Vec::new());
            full_children.pop();
            full_children.push(full_child.to_bytes());

            let hasher = PedersenHasher::<P, _>::new(None, &full_children);

            assert_eq!(full_branch, hasher.hash_with_scratch_space(&mut Vec::new()));
        }

        single_hash_consistent_inner::<Selene, Branch>(
            generate_scalars::<Helios>,
            generate_scalars::<Selene>,
        );
        single_hash_consistent_inner::<Helios, Branch>(
            generate_scalars::<Selene>,
            generate_scalars::<Helios>,
        );
        single_hash_consistent_inner::<Helios, FcmpLeafData>(
            generate_leaves,
            generate_scalars::<Helios>,
        );
    }

    #[test]
    fn chunks_hasher_consistent() {
        fn chunks_hasher_consistent_inner<P: FcmpCurve, C: Child<P>>(
            gen_children: fn(usize) -> Vec<C>,
        ) where
            CachedIncompleteBranch<P, C>: Debug + PartialEq,
        {
            let children = gen_children(10_000);

            let mut chunks_hasher = PedersenChunkHasher::<P, C>::new(None);
            let mut res = Vec::new();

            let mut i = 0;
            while i != children.len() {
                let r = u64_to_usize(rand::rngs::OsRng.next_u64()) % (children.len() - i) + 1;
                res.extend_from_slice(&chunks_hasher.hash_children(&children[i..(i + r)]));

                i += r;

                if rand::rngs::OsRng.next_u64().is_multiple_of(10) {
                    let cache = chunks_hasher.finish(None);
                    chunks_hasher = PedersenChunkHasher::new(cache);
                }
            }

            let incomplete_branch = chunks_hasher.finish(None);
            let mut chunks_hasher = PedersenChunkHasher::<P, _>::new(None);

            let res2 = chunks_hasher.hash_children(&children);
            let incomplete_branch2 = chunks_hasher.finish(None);

            assert_eq!(incomplete_branch, incomplete_branch2);
            assert_eq!(res, res2);
        }

        chunks_hasher_consistent_inner::<Selene, Branch>(generate_scalars::<Selene>);
        chunks_hasher_consistent_inner::<Helios, Branch>(generate_scalars::<Helios>);
        chunks_hasher_consistent_inner::<Selene, FcmpLeafData>(generate_leaves);
    }
}
