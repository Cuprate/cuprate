//! # Traits
//!
//! Public and private traits for Pedersen hashing and tree building.
use std::{
    io,
    ops::Mul,
    sync::{Arc, LazyLock},
};

use ciphersuite::{
    Ciphersuite,
    group::{ff::PrimeField, prime::PrimeGroup},
};
use curve25519_dalek::{EdwardsPoint, Scalar, edwards::CompressedEdwardsY};
use ec_divisors::DivisorCurve;
use helioselene::{Helios, Selene};
use monero_fcmp_plus_plus::{HELIOS_FCMP_GENERATORS, SELENE_FCMP_GENERATORS};
use monero_oxide::ed25519::{CompressedPoint, Point};

use cuprate_types::fcmp::{FcmpLeafAmount, FcmpLeafData};

use crate::Branch;

/// An internal trait for the curves used in Monero's FCMP++ tree.
pub(crate) trait FcmpCurve:
    Ciphersuite<
        F: PrimeField<Repr = [u8; 32]>,
        G: PrimeGroup<Repr = [u8; 32]>
               + DivisorCurve<FieldElement = <Self::Other as Ciphersuite>::F>,
    >
{
    /// The other curve that makes a curve cycle.
    type Other: FcmpCurve;

    /// The amount of children hashed to make a branch for this curve.
    const CHUNK_LEN: usize;

    /// The generators for Pedersen hashing.
    fn generators() -> &'static monero_fcmp_plus_plus_generators::FcmpGenerators<Self>;
}

/// A trait representing a child layer in the tree.
pub trait Child<P: FcmpCurve>: Clone + Sized + Send + Sync {
    /// The amount of parent scalars the child will make.
    const PARENT_SCALARS: usize;
    /// A hack type as Rust does not yet allow `[P::F; Self::PARENT_SCALARS]` as a return type.
    type ParentScalars: IntoIterator<Item = P::F>;

    /// Converts the child into the parent scalars for hashing.
    fn parent_scalars(&self) -> Self::ParentScalars;
}

/// A trait representing a reader for a child layer.
pub trait ChildReader<C>: Send + 'static {
    /// Get a batch of children from a given index. A size hint is given, but it does not need to be
    /// respected. If no children are returned it will be assumed the end of the child layer is reached.
    fn get_batch(&mut self, start_index: u64, batch_size_hint: u64) -> Result<Vec<C>, io::Error>;

    /// Get a specific child at the given index.
    fn get(&self, index: u64) -> Result<Option<C>, io::Error>;
}

/// A trait representing a layer writer.
pub trait LayerWriter: Send + 'static {
    /// The amount of branches already in this layer.
    fn branch_len(&self) -> Result<u64, io::Error>;

    /// Append some new branches.
    fn append_branches(&mut self, branches: &[Branch]) -> Result<(), io::Error>;

    /// Commit the new branches.
    fn commit(self) -> Result<(), io::Error>;
}

/// A trait representing a backing store for the FCMP tree.
pub trait TreeStore {
    type LayerReader: ChildReader<Branch>;
    type LayerWriter: LayerWriter;

    fn layer_reader(&self, layer_idx: usize) -> Self::LayerReader;

    fn layer_writer(&self, layer_idx: usize) -> Self::LayerWriter;
}

// impls

impl<T: TreeStore> TreeStore for Arc<T> {
    type LayerReader = T::LayerReader;
    type LayerWriter = T::LayerWriter;

    fn layer_reader(&self, layer_idx: usize) -> Self::LayerReader {
        T::layer_reader(self, layer_idx)
    }

    fn layer_writer(&self, layer_idx: usize) -> Self::LayerWriter {
        T::layer_writer(self, layer_idx)
    }
}

/// The new leaves type.
///
/// This is used for extending the FCMP tree with some new leaves to the right. Every given leaf must
/// be allowed to be inserted into the tree. i.e. no point should be the identity.
pub struct NewLeaves {
    pub leaves: Vec<FcmpLeafData>,
}

impl ChildReader<FcmpLeafData> for NewLeaves {
    fn get_batch(&mut self, _: u64, _: u64) -> Result<Vec<FcmpLeafData>, io::Error> {
        Ok(std::mem::take(&mut self.leaves))
    }

    fn get(&self, _: u64) -> Result<Option<FcmpLeafData>, io::Error> {
        unreachable!()
    }
}

impl FcmpCurve for Selene {
    type Other = Helios;

    const CHUNK_LEN: usize = 38;

    fn generators() -> &'static monero_fcmp_plus_plus_generators::FcmpGenerators<Self> {
        &SELENE_FCMP_GENERATORS
    }
}

impl FcmpCurve for Helios {
    type Other = Selene;

    const CHUNK_LEN: usize = 18;

    fn generators() -> &'static monero_fcmp_plus_plus_generators::FcmpGenerators<Self> {
        &HELIOS_FCMP_GENERATORS
    }
}

impl Child<Selene> for FcmpLeafData {
    const PARENT_SCALARS: usize = 6;
    type ParentScalars = [<Selene as Ciphersuite>::F; 6];

    fn parent_scalars(&self) -> Self::ParentScalars {
        // TODO: we can batch the computation of this which would speed up first layer hashing by around 25%
        fn clear_torsion(p: CompressedPoint) -> EdwardsPoint {
            static INV_EIGHT: LazyLock<Scalar> = LazyLock::new(|| Scalar::from(8_u8).invert());

            let p = CompressedEdwardsY::from_slice(&p.to_bytes())
                .unwrap()
                .decompress()
                .unwrap();

            if p.is_torsion_free() {
                p
            } else {
                p.mul_by_cofactor().mul(*INV_EIGHT)
            }
        }

        fn ed25519_point_to_tuple(point: EdwardsPoint) -> ([u8; 32], [u8; 32]) {
            let p =
                dalek_ff_group::EdwardsPoint::to_xy(dalek_ff_group::EdwardsPoint(point)).unwrap();

            (p.0.to_repr(), p.1.to_repr())
        }

        let o = clear_torsion(CompressedPoint::from(self.o));

        let c = match self.c {
            // A clear amt, we can create a 0 commitment which we do not need to check for torsion
            FcmpLeafAmount::Clear(amt) => cuprate_helper::crypto::compute_zero_commitment(amt)
                .decompress()
                .unwrap(),
            FcmpLeafAmount::Commitment(cmt) => {
                // A commitment to an amt, check it for torsion.
                clear_torsion(CompressedPoint::from(cmt))
            }
        };

        let i = Point::biased_hash(self.o).into();

        let o = ed25519_point_to_tuple(o);
        let i = ed25519_point_to_tuple(i);
        let c = ed25519_point_to_tuple(c);

        let f = |c| <Selene as Ciphersuite>::F::from_repr(c).unwrap();

        [f(o.0), f(o.1), f(i.0), f(i.1), f(c.0), f(c.1)]
    }
}

impl<P: FcmpCurve> Child<P> for Branch {
    const PARENT_SCALARS: usize = 1;
    type ParentScalars = [P::F; 1];

    fn parent_scalars(&self) -> Self::ParentScalars {
        #[inline(always)]
        fn clear_sign(mut x: [u8; 32]) -> [u8; 32] {
            x[31] &= u8::MAX >> 1;
            x
        }

        let child = clear_sign(*self);

        [P::F::from_repr_vartime(child).unwrap()]
    }
}
