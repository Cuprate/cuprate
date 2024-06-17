//! Database [table](crate::tables) types.
//!
//! This module contains all types used by the database tables,
//! and aliases for common Monero-related types that use the
//! same underlying primitive type.
//!
//! <!-- FIXME: Add schema here or a link to it when complete -->

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
use bytemuck::{Pod, Zeroable};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_database::StorableVec;

//---------------------------------------------------------------------------------------------------- Aliases
// These type aliases exist as many Monero-related types are the exact same.
// For clarity, they're given type aliases as to not confuse them.

/// An output's amount.
pub type Amount = u64;

/// The index of an [`Amount`] in a list of duplicate `Amount`s.
pub type AmountIndex = u64;

/// A list of [`AmountIndex`]s.
pub type AmountIndices = StorableVec<AmountIndex>;

/// A serialized block.
pub type BlockBlob = StorableVec<u8>;

/// A block's hash.
pub type BlockHash = [u8; 32];

/// A block's height.
pub type BlockHeight = u64;

/// A key image.
pub type KeyImage = [u8; 32];

/// Pruned serialized bytes.
pub type PrunedBlob = StorableVec<u8>;

/// A prunable serialized bytes.
pub type PrunableBlob = StorableVec<u8>;

/// A prunable hash.
pub type PrunableHash = [u8; 32];

/// A serialized transaction.
pub type TxBlob = StorableVec<u8>;

/// A transaction's global index, or ID.
pub type TxId = u64;

/// A transaction's hash.
pub type TxHash = [u8; 32];

/// The unlock time value of an output.
pub type UnlockTime = u64;

//---------------------------------------------------------------------------------------------------- BlockInfoV1
/// A identifier for a pre-RCT [`Output`].
///
/// This can also serve as an identifier for [`RctOutput`]'s
/// when [`PreRctOutputId::amount`] is set to `0`, although,
/// in that case, only [`AmountIndex`] needs to be known.
///
/// This is the key to the [`Outputs`](crate::tables::Outputs) table.
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
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
/// # use cuprate_blockchain::types::*;
/// # use std::mem::*;
/// assert_eq!(size_of::<PreRctOutputId>(), 16);
/// assert_eq!(align_of::<PreRctOutputId>(), 8);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct PreRctOutputId {
    /// Amount of the output.
    ///
    /// This should be `0` if the output is an [`RctOutput`].
    pub amount: Amount,
    /// The index of the output with the same `amount`.
    ///
    /// In the case of [`Output`]'s, this is the index of the list
    /// of outputs with the same clear amount.
    ///
    /// In the case of [`RctOutput`]'s, this is the
    /// global index of _all_ `RctOutput`s
    pub amount_index: AmountIndex,
}

//---------------------------------------------------------------------------------------------------- BlockInfoV3
/// Block information.
///
/// This is the value in the [`BlockInfos`](crate::tables::BlockInfos) table.
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// // Assert Storable is correct.
/// let a = BlockInfo {
///     timestamp: 1,
///     cumulative_generated_coins: 123,
///     weight: 321,
///     cumulative_difficulty_low: 112,
///     cumulative_difficulty_high: 112,
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
/// # use cuprate_blockchain::types::*;
/// # use std::mem::*;
/// assert_eq!(size_of::<BlockInfo>(), 88);
/// assert_eq!(align_of::<BlockInfo>(), 8);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct BlockInfo {
    /// The UNIX time at which the block was mined.
    pub timestamp: u64,
    /// The total amount of coins mined in all blocks so far, including this block's.
    pub cumulative_generated_coins: u64,
    /// The adjusted block size, in bytes.
    ///
    /// See [`block_weight`](https://monero-book.cuprate.org/consensus_rules/blocks/weights.html#blocks-weight).
    pub weight: u64,
    /// Least-significant 64 bits of the 128-bit cumulative difficulty.
    pub cumulative_difficulty_low: u64,
    /// Most-significant 64 bits of the 128-bit cumulative difficulty.
    pub cumulative_difficulty_high: u64,
    /// The block's hash.
    pub block_hash: [u8; 32],
    /// The total amount of RCT outputs so far, including this block's.
    pub cumulative_rct_outs: u64,
    /// The long term block weight, based on the median weight of the preceding `100_000` blocks.
    ///
    /// See [`long_term_weight`](https://monero-book.cuprate.org/consensus_rules/blocks/weights.html#long-term-block-weight).
    pub long_term_weight: u64,
}

//---------------------------------------------------------------------------------------------------- OutputFlags
bitflags::bitflags! {
    /// Bit flags for [`Output`]s and [`RctOutput`]s,
    ///
    /// Currently only the first bit is used and, if set,
    /// it means this output has a non-zero unlock time.
    ///
    /// ```rust
    /// # use std::borrow::*;
    /// # use cuprate_blockchain::{*, types::*};
    /// // Assert Storable is correct.
    /// let a = OutputFlags::NON_ZERO_UNLOCK_TIME;
    /// let b = Storable::as_bytes(&a);
    /// let c: OutputFlags = Storable::from_bytes(b);
    /// assert_eq!(a, c);
    /// ```
    ///
    /// # Size & Alignment
    /// ```rust
    /// # use cuprate_blockchain::types::*;
    /// # use std::mem::*;
    /// assert_eq!(size_of::<OutputFlags>(), 4);
    /// assert_eq!(align_of::<OutputFlags>(), 4);
    /// ```
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
    #[repr(transparent)]
    pub struct OutputFlags: u32 {
        /// This output has a non-zero unlock time.
        const NON_ZERO_UNLOCK_TIME = 0b0000_0001;
    }
}

//---------------------------------------------------------------------------------------------------- Output
/// A pre-RCT (v1) output's data.
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// // Assert Storable is correct.
/// let a = Output {
///     key: [1; 32],
///     height: 1,
///     output_flags: OutputFlags::empty(),
///     tx_idx: 3,
/// };
/// let b = Storable::as_bytes(&a);
/// let c: Output = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_blockchain::types::*;
/// # use std::mem::*;
/// assert_eq!(size_of::<Output>(), 48);
/// assert_eq!(align_of::<Output>(), 8);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct Output {
    /// The public key of the output.
    pub key: [u8; 32],
    /// The block height this output belongs to.
    // PERF: We could get this from the tx_idx with the `TxHeights`
    // table but that would require another look up per out.
    pub height: u32,
    /// Bit flags for this output.
    pub output_flags: OutputFlags,
    /// The index of the transaction this output belongs to.
    pub tx_idx: u64,
}

//---------------------------------------------------------------------------------------------------- RctOutput
/// An RCT (v2+) output's data.
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// // Assert Storable is correct.
/// let a = RctOutput {
///     key: [1; 32],
///     height: 1,
///     output_flags: OutputFlags::empty(),
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
/// # use cuprate_blockchain::types::*;
/// # use std::mem::*;
/// assert_eq!(size_of::<RctOutput>(), 80);
/// assert_eq!(align_of::<RctOutput>(), 8);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct RctOutput {
    /// The public key of the output.
    pub key: [u8; 32],
    /// The block height this output belongs to.
    // PERF: We could get this from the tx_idx with the `TxHeights`
    // table but that would require another look up per out.
    pub height: u32,
    /// Bit flags for this output, currently only the first bit is used and, if set, it means this output has a non-zero unlock time.
    pub output_flags: OutputFlags,
    /// The index of the transaction this output belongs to.
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
