//! Bucket data structure
//!
//! A collection data structure that discriminates its unique items and place them into "buckets".
//!
//! The item must implement the [`Bucketable`] trait that defines how to create the discriminant
//! from the item type. The data structure will internally contain any item into "buckets" or vectors
//! of sized capacity `N` that regroup all the stored items with this specific discriminant.
//!
//! A practical example of this data structure is for storing `N` amount of IP discriminated by their subnets.
//! You can store in each "buckets" corresponding to a `/16` subnet up to `N` IPs of that subnet.
//!
//! # Example
//!
//! ```
//! use cuprate_p2p_bucket::BucketSet;
//! use std::net::Ipv4Addr;
//!
//! // Create a new bucket that can store at most 2 IPs in a particular `/16` subnet.
//! let mut bucket = BucketSet::<2,Ipv4Addr>::new();
//!
//! // Fulfill the `96.96.0.0/16` bucket.
//! bucket.push("96.96.0.1".parse().unwrap());
//! bucket.push("96.96.0.2".parse().unwrap());
//! assert_eq!(2, bucket.len());
//! assert_eq!(2, bucket.len_bucket(&[96_u8,96_u8]).unwrap());
//!
//! // Push a new IP from another subnet
//! bucket.push("127.0.0.1".parse().unwrap());
//! assert_eq!(3, bucket.len());
//! assert_eq!(2, bucket.len_bucket(&[96_u8,96_u8]).unwrap());
//! assert_eq!(1, bucket.len_bucket(&[127_u8,0_u8]).unwrap());
//!
//! // Attempting to push a new IP within `96.96.0.0/16` bucket will return the IP back
//! // as this subnet is already full.
//! let pushed = bucket.push("96.96.0.3".parse().unwrap());
//! assert!(pushed.is_some());
//! assert_eq!(2, bucket.len_bucket(&[96_u8,96_u8]).unwrap());
//!
//! ```
use arrayvec::{ArrayVec, CapacityError};
use cuprate_p2p_core::services::ZoneSpecificPeerListEntryBase;
use cuprate_p2p_core::NetZoneAddress;
use cuprate_wire::OnionAddr;
use rand::prelude::IteratorRandom;
use rand::{random, Rng};
use std::fmt::Display;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::ops::Not;
use std::{collections::BTreeMap, net::Ipv4Addr};

/// A discriminant that can be computed from the type.
pub trait Bucketable: Sized + Eq + Clone {
    /// The type of the discriminant being used in the Binary tree.
    type Discriminant: Send + Ord;

    /// Method that can compute the discriminant from the item.
    fn discriminant(&self) -> Self::Discriminant;
}

/// A collection data structure discriminating its unique items
/// with a specified method. Limiting the amount of items stored
/// with that discriminant to the const `N`.
#[derive(Debug)]
pub struct BucketMap<const N: usize, I: Bucketable> {
    /// The storage of the bucket
    storage: BTreeMap<I::Discriminant, ArrayVec<I, N>>,
}

impl<const N: usize, I: Bucketable> BucketMap<N, I> {
    /// Create a new [`BucketMap`]
    pub const fn new() -> Self {
        Self {
            storage: BTreeMap::new(),
        }
    }

    /// Push a new element into the Bucket
    ///
    /// Will internally create a new vector for each new discriminant being
    /// generated from an item.
    ///
    /// This function WILL NOT push the element if it already exists.
    ///
    /// Return `None` if the item has been pushed or ignored. `Some(I)` if
    /// the vector is full.
    ///
    /// # Example
    ///
    /// ```
    /// use cuprate_p2p_bucket::BucketSet;
    /// use std::net::Ipv4Addr;
    ///
    /// let mut bucket = BucketSet::<8,Ipv4Addr>::new();
    ///
    /// // Push a first IP address.
    /// bucket.push("127.0.0.1".parse().unwrap());
    /// assert_eq!(1, bucket.len());
    ///
    /// // Push the same IP address a second time.
    /// bucket.push("127.0.0.1".parse().unwrap());
    /// assert_eq!(1, bucket.len());
    /// ```
    #[must_use]
    pub fn push(&mut self, item: I) -> Option<I> {
        let discriminant = item.discriminant();

        if let Some(vec) = self.storage.get_mut(&discriminant) {
            // Push the item if it doesn't exist.
            if !vec.contains(&item) {
                return vec.try_push(item).err().map(CapacityError::element);
            }
        } else {
            // Initialize the vector if not found.
            let mut vec = ArrayVec::<I, N>::new();
            vec.push(item);
            self.storage.insert(discriminant, vec);
        }

        None
    }

    pub fn get_mut<R>(&mut self, item: &R) -> Option<&mut I>
    where
        R: Bucketable<Discriminant = I::Discriminant>,
        I: PartialEq<R>,
    {
        self.storage
            .get_mut(&item.discriminant())
            .and_then(|vec| vec.iter_mut().find_map(|v| (v == item).then_some(v)))
    }

    pub fn contains_key<R>(&self, item: &R) -> bool
    where
        R: Bucketable<Discriminant = I::Discriminant>,
        I: PartialEq<R>,
    {
        self.storage
            .get(&item.discriminant())
            .is_some_and(|vec| vec.iter().any(|v| (v == item)))
    }

    /// Will attempt to remove an item from the bucket.
    pub fn remove<R>(&mut self, item: &R) -> Option<I>
    where
        R: Bucketable<Discriminant = I::Discriminant>,
        I: PartialEq<R>,
    {
        self.storage.get_mut(&item.discriminant()).and_then(|vec| {
            vec.iter()
                .enumerate()
                .find_map(|(i, v)| (v == item).then_some(i))
                .map(|index| vec.swap_remove(index))
        })
    }

    /// Return the number of item stored within the storage
    pub fn len(&self) -> usize {
        self.storage.values().map(ArrayVec::len).sum()
    }

    /// Return the number of item stored with a specific discriminant.
    ///
    /// This method returns None if the bucket with this discriminant
    /// doesn't exist.
    pub fn len_bucket(&self, discriminant: &I::Discriminant) -> Option<usize> {
        self.storage.get(discriminant).map(ArrayVec::len)
    }

    /// Return `true` if the storage contains no items
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return a reference to an item chosen at random.
    ///
    /// Repeated use of this function will provide a normal distribution of
    /// items based on their discriminants.
    pub fn take_random<R, Rng>(
        &mut self,
        current_set: &[R],
        no_matching_discriminant: bool,
        r: &mut Rng,
    ) -> Option<I>
    where
        R: Bucketable<Discriminant = I::Discriminant>,
        I: PartialEq<R>,
        Rng: rand::Rng,
    {
        let len = self.storage.len();
        let current_set_discriminant: Vec<_> =
            current_set.iter().map(Bucketable::discriminant).collect();

        if len == 0 {
            return None;
        }

        self.storage
            .iter_mut()
            .skip(r.gen_range(0..len))
            .take(len)
            .find_map(|(discriminant, vec)| {
                if no_matching_discriminant && current_set_discriminant.contains(discriminant) {
                    return None;
                }

                let no_match = vec
                    .iter_mut()
                    .enumerate()
                    .filter_map(|(i, e)| {
                        current_set.iter().find(|c| e == *c).is_none().then_some(i)
                    })
                    .choose(r);

                no_match.map(|i| vec.swap_remove(i))
            })
    }

    pub fn values(&self) -> impl Iterator<Item = &I> {
        self.storage.values().flat_map(|v| v.iter())
    }
}

impl<const N: usize, I: Bucketable> Default for BucketMap<N, I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: NetZoneAddress + Bucketable> Bucketable for ZoneSpecificPeerListEntryBase<A> {
    type Discriminant = A::Discriminant;

    fn discriminant(&self) -> Self::Discriminant {
        self.adr.discriminant()
    }
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
#[expect(variant_size_differences)]
pub enum IpDiscriminant {
    V4([u8; 2]),
    V6([u8; 8]),
}

impl AsRef<[u8]> for IpDiscriminant {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::V4(x) => x.as_ref(),
            Self::V6(x) => x.as_ref(),
        }
    }
}

impl Bucketable for Ipv4Addr {
    /// We are discriminating by `/16` subnets.
    type Discriminant = IpDiscriminant;

    fn discriminant(&self) -> Self::Discriminant {
        IpDiscriminant::V4([self.octets()[0], self.octets()[1]])
    }
}

impl Bucketable for Ipv6Addr {
    /// We are discriminating by `/16` subnets.
    type Discriminant = IpDiscriminant;

    fn discriminant(&self) -> Self::Discriminant {
        IpDiscriminant::V6(self.octets()[0..8].try_into().unwrap())
    }
}

impl Bucketable for SocketAddr {
    /// We are discriminating by `/16` subnets.
    type Discriminant = IpDiscriminant;

    fn discriminant(&self) -> Self::Discriminant {
        match self.ip() {
            IpAddr::V4(v) => v.discriminant(),
            IpAddr::V6(v) => v.discriminant(),
        }
    }
}

impl Bucketable for OnionAddr {
    /// We are discriminating by `/16` subnets.
    type Discriminant = [u8; 56];

    fn discriminant(&self) -> Self::Discriminant {
        self.domain()
    }
}
