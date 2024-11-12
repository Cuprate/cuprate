//! `BucketMap` data structure
//!
//! A collection data structure that discriminates its unique items and place them into "buckets".
//!
//! The item must implement the [`Bucketable`] trait that defines how to create the discriminant
//! from the item type and its discriminated field. The data structure will internally contain any item into 
//! "buckets" or vectors of sized capacity `N` that regroup all the stored items with this specific discriminant.
//!
//! A practical example of this data structure is for storing `N` amount of IP discriminated by their subnets.
//! You can store in each "buckets" corresponding to a `/16` subnet up to `N` IPs of that subnet.
//!
//! # Example
//!
//! ```
//! use cuprate_p2p_bucket::BucketMap;
//! use std::net::Ipv4Addr;
//!
//! // Create a new bucket that can store at most 2 IPs in a particular `/16` subnet.
//! let mut bucket = BucketMap::<2,Ipv4Addr>::new();
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
use rand::Rng;

use std::{
    collections::BTreeMap,
    fmt::Debug, 
    net::{IpAddr, SocketAddr}
};

/// This trait defines an item key and discriminant in
/// a `BucketMap` and method to compute the discriminant
/// from the key
pub trait Bucketable: Sized + Eq + Clone {
    /// The type of the key being used to lookup for a specific element.
    /// It is also the type from which the discriminant is computed.
    type Key: Sized + Eq + Clone;

    /// The type of discriminant used to discriminate items together.
    type Discriminant: Ord + Debug + Send;
    
    /// Method that give return the key from the item.
    fn key(&self) -> Self::Key;
    
    /// Method for computing a discriminant from the key of the item.
    fn compute_discriminant(key: &Self::Key) -> Self::Discriminant;
    
    /// Method that can compute the discriminant directly from the item.
    fn discriminant(&self) -> Self::Discriminant {
        Self::compute_discriminant(&self.key())
    }
}

/// A collection data structure discriminating its unique items
/// with a specified method. Limiting the amount of items stored
/// with that discriminant to the const `N`.
#[derive(Debug)]
pub struct BucketMap<I: Bucketable, const N: usize> {
    /// The storage of the bucket
    storage: BTreeMap<I::Discriminant, ArrayVec<I, N>>,
}

impl<I: Bucketable, const N: usize> BucketMap<I, N> {
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
    /// use cuprate_p2p_bucket::BucketMap;
    /// use std::net::Ipv4Addr;
    ///
    /// let mut bucket = BucketMap::<8,Ipv4Addr>::new();
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
    
    /// Get a reference to an item stored in the `BucketMap` with 
    /// the specified item key.
    pub fn get(&self, key: &I::Key) -> Option<&I> {
        self.storage.get(&I::compute_discriminant(key)).and_then(|vec| {
            vec.iter()
                .enumerate()
                .find_map(|(i, v)| (*key == v.key()).then_some(i))
                .and_then(|index| vec.get(index))
        })
    }
    
    /// Get a mutable reference to an item stored in the `BucketMap`
    /// with the specified item key.
    pub fn get_mut(&mut self, key: &I::Key) -> Option<&mut I> {
        self.storage.get_mut(&I::compute_discriminant(key)).and_then(|vec| {
            vec.iter()
                .enumerate()
                .find_map(|(i, v)| (*key == v.key()).then_some(i))
                .and_then(|index| vec.get_mut(index))
        })
    }
    
    /// Return a reference to an item chosen at random.
    ///
    /// Repeated use of this function will provide a normal distribution of
    /// items based on their discriminants.
    pub fn get_random<R: Rng>(&self, r: &mut R) -> Option<&I> {
        // Get the total amount of discriminants to explore.
        let len = self.storage.len();

        // Get a random bucket.
        let (_, vec) = self.storage.iter().nth(r.r#gen::<usize>() / len).unwrap();

        // Return a reference chose at random.
        vec.get(r.r#gen::<usize>() / vec.len())
    }
    
    /// Will attempt to remove an item from the bucket.
    /// 
    /// Internally use `swap_remove` which breaks the order in which
    /// elements in a bucket were inserted.
    pub fn remove(&mut self, key: &I::Key) -> Option<I> {
        self.storage.get_mut(&I::compute_discriminant(key)).and_then(|vec| {
            vec.iter()
                .enumerate()
                .find_map(|(i, v)| (*key == v.key()).then_some(i))
                .map(|index| vec.swap_remove(index))
        })
    }
    
    /// Return `true` if the `BucketMap` contains an item
    /// with the key `key`. Return `false` otherwise.
    pub fn contains(&self, key: &I::Key) -> bool {
        self.get(key).is_some()
    }

    /// Return the number of item stored within the storage.
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

    /// Return `true` if the storage contains no items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Return an iterator of the keys of all items contained in
    /// the `BucketMap`.
    pub fn keys(&self) -> impl Iterator<Item = I::Key> + '_ {
        
        self.storage
            .iter()
            .flat_map(|bucket| 
                bucket.1
                    .iter()
                    .map(I::key)
            )
    }
    
    /// Return an iterator of all items contained in
    /// the `BucketMap`.
    pub fn values(&self) -> impl Iterator<Item = &I> + '_ {
        
        self.storage
            .iter()
            .flat_map(|bucket| 
                bucket.1
                    .iter()
            )
    }
}

impl<I: Bucketable, const N: usize> Default for BucketMap<I, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// A discriminant type for `SocketAddr`.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum IpSubnet {
    /// The discriminant for IPv4 subnets is the 16 bytes prefix (therefore discriminating `/16` subnets)
    V4([u8; 2]),
    /// The equivalent of a `/16` IPv4 subnet for IPv6 is a `/32` prefix
    V6([u8; 4])
}

impl Bucketable for SocketAddr {
    
    /// The key is the IP address itself.
    type Key = IpAddr;
    
    /// The discriminant is either a truncated `/16` IPv4 or a `/32` IPv6 prefix.
    type Discriminant = IpSubnet;
    
    #[inline]
    fn key(&self) -> Self::Key {
        self.ip()
    }
    
    #[inline]
    fn compute_discriminant(key: &Self::Key) -> Self::Discriminant {
        match key {
            IpAddr::V4(ip_addr_v4) => {
                let bytes = ip_addr_v4.octets();
                IpSubnet::V4([bytes[0],bytes[1]])
                
            },
            IpAddr::V6(ip_addr_v6) => {
                let bytes = ip_addr_v6.octets();
                IpSubnet::V6([bytes[0],bytes[1],bytes[2],bytes[3]])
            }
        }
    }
}

#[cfg(test)]
mod test {
    
}
