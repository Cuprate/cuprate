//! Tx-pool Database tables.
//!
//! # Table marker structs
//! This module contains all the table definitions used by [`cuprate_txpool`](crate).
//!
//! The zero-sized structs here represents the table type;
//! they all are essentially marker types that implement [`cuprate_database::Table`].
//!
//! Table structs are `CamelCase`, and their static string
//! names used by the actual database backend are `snake_case`.
//!
//! For example: [`TransactionBlobs`] -> `transaction_blobs`.
//!
//! # Traits
//! This module also contains a set of traits for
//! accessing _all_ tables defined here at once.
use cuprate_database::{define_tables, StorableStr, StorableVec};

use crate::types::{
    DatabaseVersion, KeyImage, RawCachedVerificationState, TransactionBlobHash, TransactionHash,
    TransactionInfo,
};

define_tables! {
    /// Serialized transaction blobs.
    ///
    /// This table contains the transaction blobs of all the transactions in the pool.
    0 => TransactionBlobs,
    TransactionHash => StorableVec<u8>,

    /// Transaction information.
    ///
    /// This table contains information of all transactions currently in the pool.
    1 => TransactionInfos,
    TransactionHash => TransactionInfo,

    /// Cached transaction verification state.
    ///
    /// This table contains the cached verification state of all translations in the pool.
    2 => CachedVerificationState,
    TransactionHash => RawCachedVerificationState,

    /// Spent key images.
    ///
    /// This table contains the spent key images from all transactions in the pool.
    3 => SpentKeyImages,
    KeyImage => TransactionHash,

    /// Transaction blob hashes that are in the pool.
    4 => KnownBlobHashes,
    TransactionBlobHash => TransactionHash,

    /// Current database version.
    5 => Metadata,
    StorableStr => DatabaseVersion,
}
