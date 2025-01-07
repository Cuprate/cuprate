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
//! use cuprate_p2p_bucket::Bucket;
//! use std::net::Ipv4Addr;
//!
//! // Create a new bucket that can store at most 2 IPs in a particular `/16` subnet.
//! let mut bucket = Bucket::<2,Ipv4Addr>::new();
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
use rand::random;

use std::{collections::BTreeMap, net::Ipv4Addr};

/// A discriminant that can be computed from the type.
pub trait Bucketable: Sized + Eq + Clone {
    /// The type of the discriminant being used in the Binary tree.
    type Discriminant: Ord + AsRef<[u8]>;

    /// Method that can compute the discriminant from the item.
    fn discriminant(&self) -> Self::Discriminant;
}

/// A collection data structure discriminating its unique items
/// with a specified method. Limiting the amount of items stored
/// with that discriminant to the const `N`.
pub struct Bucket<const N: usize, I: Bucketable> {
    /// The storage of the bucket
    storage: BTreeMap<I::Discriminant, ArrayVec<I, N>>,
}

impl<const N: usize, I: Bucketable> Bucket<N, I> {
    /// Create a new Bucket
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
    /// use cuprate_p2p_bucket::Bucket;
    /// use std::net::Ipv4Addr;
    ///
    /// let mut bucket = Bucket::<8,Ipv4Addr>::new();
    ///
    /// // Push a first IP address.
    /// bucket.push("127.0.0.1".parse().unwrap());
    /// assert_eq!(1, bucket.len());
    ///
    /// // Push the same IP address a second time.
    /// bucket.push("127.0.0.1".parse().unwrap());
    /// assert_eq!(1, bucket.len());
    /// ```
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

    /// Will attempt to remove an item from the bucket.
    pub fn remove(&mut self, item: &I) -> Option<I> {
        self.storage.get_mut(&item.discriminant()).and_then(|vec| {
            vec.iter()
                .enumerate()
                .find_map(|(i, v)| (item == v).then_some(i))
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
    pub fn get_random(&mut self) -> Option<&I> {
        // Get the total amount of discriminants to explore.
        let len = self.storage.len();

        // Get a random bucket.
        let (_, vec) = self.storage.iter().nth(random::<usize>() / len).unwrap();

        // Return a reference chose at random.
        vec.get(random::<usize>() / vec.len())
    }
}

impl<const N: usize, I: Bucketable> Default for Bucket<N, I> {
    fn default() -> Self {
        Self::new()
    }
}

impl Bucketable for Ipv4Addr {
    /// We are discriminating by `/16` subnets.
    type Discriminant = [u8; 2];

    fn discriminant(&self) -> Self::Discriminant {
        [self.octets()[0], self.octets()[1]]
    }
}
