//! Database [table](crate::tables) types.
//!
//! This module contains all types used by the database tables.
//!
//! TODO: Add schema here or a link to it.

/*
 * <============================================> VERY BIG SCARY SAFETY MESSAGE <============================================>
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 *
 *
 *
 *                                We use `bytemuck` to (de)serialize data types in the database.
 *                          We are SAFELY casting bytes, but to do so, we must uphold some invariants.
 *                          When editing this file, there is only 1 commandment that MUST be followed:
 *
 *                                   1. Thou shall only utilize `bytemuck`'s derive macros
 *
 *                             The derive macros will fail at COMPILE time if something is incorrect.
 *                                  <https://docs.rs/bytemuck/latest/bytemuck/derive.Pod.html>
 *                                 If you submit a PR that breaks this I will come and find you.
 *
 *
 *
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * <============================================> VERY BIG SCARY SAFETY MESSAGE <============================================>
 */
// actually i still don't trust you. no unsafe.
#![forbid(unsafe_code)] // if you remove this line i will steal your monero

//---------------------------------------------------------------------------------------------------- Import
use bytemuck::{AnyBitPattern, NoUninit, Pod, Zeroable};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::storable::StorableVec;

//---------------------------------------------------------------------------------------------------- Aliases
// TODO: document these, why they exist, and their purpose.
//
// Notes:
// - Keep this sorted A-Z

/// TODO
pub type Amount = u64;

/// TODO
pub type AmountIndex = u64;

/// TODO
pub type AmountIndices = StorableVec<AmountIndex>;

/// TODO
pub type BlockBlob = StorableVec<u8>;

/// TODO
pub type BlockHash = [u8; 32];

/// TODO
pub type BlockHeight = u64;

/// TODO
pub type KeyImage = [u8; 32];

/// TODO
pub type PrunedBlob = StorableVec<u8>;

/// TODO
pub type PrunableBlob = StorableVec<u8>;

/// TODO
pub type PrunableHash = [u8; 32];

/// TODO
pub type TxBlob = StorableVec<u8>;

/// TODO
pub type TxId = u64;

/// TODO
pub type TxHash = [u8; 32];

/// TODO
pub type UnlockTime = u64;

//---------------------------------------------------------------------------------------------------- BlockInfoV1
/// TODO
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_database::{*, types::*};
/// // Assert Storable is correct.
/// let a = PreRctOutputId {
///     amount: 1,
///     amount_index: 123,
/// };
/// let b = Storable::as_bytes(&a);
/// let c: PreRctOutputId = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_database::types::*;
/// # use std::mem::*;
/// assert_eq!(size_of::<PreRctOutputId>(), 16);
/// assert_eq!(align_of::<PreRctOutputId>(), 8);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct PreRctOutputId {
    /// TODO
    pub amount: Amount,
    /// TODO
    pub amount_index: AmountIndex,
}

//---------------------------------------------------------------------------------------------------- BlockInfoV3
/// TODO
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_database::{*, types::*};
/// // Assert Storable is correct.
/// let a = BlockInfo {
///     timestamp: 1,
///     total_generated_coins: 123,
///     weight: 321,
///     cumulative_difficulty: 112,
///     block_hash: [54; 32],
///     cumulative_rct_outs: 2389,
///     long_term_weight: 2389,
/// };
/// let b = Storable::as_bytes(&a);
/// let c: BlockInfo = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_database::types::*;
/// # use std::mem::*;
/// assert_eq!(size_of::<BlockInfo>(), 88);
/// assert_eq!(align_of::<BlockInfo>(), 8);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct BlockInfo {
    /// TODO
    pub timestamp: u64,
    /// TODO
    pub total_generated_coins: u64,
    /// TODO
    pub weight: u64,
    /// TODO
    pub cumulative_difficulty: u128,
    /// TODO
    pub block_hash: [u8; 32],
    /// TODO
    pub cumulative_rct_outs: u64,
    /// TODO
    pub long_term_weight: u64,
}

//---------------------------------------------------------------------------------------------------- Output
/// TODO
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_database::{*, types::*};
/// // Assert Storable is correct.
/// let a = Output {
///     key: [1; 32],
///     height: 1,
///     output_flags: 0,
///     tx_idx: 3,
/// };
/// let b = Storable::as_bytes(&a);
/// let c: Output = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_database::types::*;
/// # use std::mem::*;
/// assert_eq!(size_of::<Output>(), 48);
/// assert_eq!(align_of::<Output>(), 8);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct Output {
    /// TODO
    pub key: [u8; 32],
    /// We could get this from the tx_idx with the Tx Heights table but that would require another look up per out.
    pub height: u32,
    /// Bit flags for this output, currently only the first bit is used and, if set, it means this output has a non-zero unlock time.
    pub output_flags: u32,
    /// TODO
    pub tx_idx: u64,
}

// bitflags::bitflags! {
//     /// TODO
//     #[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
//     #[repr(C)]
//     pub struct OutputFlags: u32 {
//         /// No flags set.
//         const NONE = 0b0000_0000;
//         /// This output has a non-zero unlock time.
//         const NON_ZERO_UNLOCK_TIME = 0b0000_0001;
//     }
// }

//---------------------------------------------------------------------------------------------------- RctOutput
/// TODO
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_database::{*, types::*};
/// // Assert Storable is correct.
/// let a = RctOutput {
///     key: [1; 32],
///     height: 1,
///     output_flags: 0,
///     tx_idx: 3,
///     commitment: [3; 32],
/// };
/// let b = Storable::as_bytes(&a);
/// let c: RctOutput = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_database::types::*;
/// # use std::mem::*;
/// assert_eq!(size_of::<RctOutput>(), 80);
/// assert_eq!(align_of::<RctOutput>(), 8);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct RctOutput {
    /// TODO
    pub key: [u8; 32],
    /// We could get this from the tx_idx with the Tx Heights table but that would require another look up per out.
    pub height: u32,
    /// Bit flags for this output, currently only the first bit is used and, if set, it means this output has a non-zero unlock time.
    pub output_flags: u32,
    /// TODO
    pub tx_idx: u64,
    /// The amount commitment of this output.
    pub commitment: [u8; 32],
}
// TODO: local_index?

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
