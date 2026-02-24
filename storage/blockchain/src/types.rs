//! Blockchain [table](crate::tables) types.
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

use std::borrow::Cow;
use std::cmp::Ordering;
use std::marker::PhantomData;
//---------------------------------------------------------------------------------------------------- Import
use std::num::NonZero;

use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_types::{Chain, ChainId};

//---------------------------------------------------------------------------------------------------- Aliases
// These type aliases exist as many Monero-related types are the exact same.
// For clarity, they're given type aliases as to not confuse them.

/// An output's amount.
pub type Amount = u64;

/// The index of an [`Amount`] in a list of duplicate `Amount`s.
pub type AmountIndex = u64;

/// A list of [`AmountIndex`]s.
pub type AmountIndices = Vec<AmountIndex>;

/// A block's hash.
pub type BlockHash = [u8; 32];

/// A block's height.
pub type BlockHeight = usize;

/// A key image.
pub type KeyImage = [u8; 32];

/// A prunable hash.
pub type PrunableHash = [u8; 32];

/// A transaction's global index, or ID.
pub type TxId = u64;

/// A transaction's hash.
pub type TxHash = [u8; 32];

/// The unlock time value of an output.
pub type UnlockTime = u64;

/// Information on a transaction in the blockchain.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct TxInfo {
    /// The height of this transaction.
    pub height: usize,
    /// The index of the transactions pruned blob in the pruned tape.
    pub pruned_blob_idx: u64,
    /// The index of the transactions prunable blob in the corresponding prunable tape.
    pub prunable_blob_idx: u64,
    /// The size of the transactions pruned blob.
    pub pruned_size: usize,
    /// The size of th transaction prunable blob.
    pub prunable_size: usize,
    /// The index of the first V2 output in this transaction.
    ///
    /// will be [`u64::MAX`] for V1 transactions.
    pub rct_output_start_idx: u64,
    /// The number of RCT outputs in this transaction.
    ///
    /// Undefined for V1 transactions.
    pub numb_rct_outputs: usize,
}

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
/// use cuprate_database::Storable;
///
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
/// use cuprate_database::Storable;
///
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
///     mining_tx_index: 23
/// };
/// let b = Storable::as_bytes(&a);
/// let c: BlockInfo = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_blockchain::types::*;
/// assert_eq!(size_of::<BlockInfo>(), 96);
/// assert_eq!(align_of::<BlockInfo>(), 8);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable, Default)]
#[repr(C)]
pub struct BlockInfo {
    /// The total amount of coins mined in all blocks so far, including this block's.
    pub cumulative_generated_coins: u64,
    /// The adjusted block size, in bytes.
    ///
    /// See [`block_weight`](https://monero-book.cuprate.org/consensus_rules/blocks/weights.html#blocks-weight).
    pub weight: usize,
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
    pub long_term_weight: usize,
    /// [`TxId`] (u64) of the block coinbase transaction.
    pub mining_tx_index: TxId,

    pub prunable_blob_idx: u64,
    pub v1_prunable_blob_idx: u64,
    pub pruned_blob_idx: u64,
}

//---------------------------------------------------------------------------------------------------- Output
/// A pre-RCT (v1) output's data.
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// use cuprate_database::Storable;
///
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
    pub height: usize,
    /// The time lock of this output.
    pub timelock: u64,
    /// The index of the transaction this output belongs to.
    pub tx_idx: TxId,
}

//---------------------------------------------------------------------------------------------------- RctOutput
/// An RCT (v2+) output's data.
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// use cuprate_database::Storable;
///
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
    pub height: usize,
    /// The time lock of this output.
    pub timelock: u64,
    /// The index of the transaction this output belongs to.
    pub tx_idx: TxId,
    /// The amount commitment of this output.
    pub commitment: [u8; 32],
}
// TODO: local_index?

//---------------------------------------------------------------------------------------------------- RawChain
/// [`Chain`] in a format which can be stored in the DB.
///
/// Implements [`Into`] and [`From`] for [`Chain`].
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// use cuprate_database::Storable;
/// use cuprate_types::Chain;
///
/// // Assert Storable is correct.
/// let a: RawChain = Chain::Main.into();
/// let b = Storable::as_bytes(&a);
/// let c: RawChain = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_blockchain::types::*;
/// assert_eq!(size_of::<RawChain>(), 8);
/// assert_eq!(align_of::<RawChain>(), 8);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct RawChain(u64);

impl From<Chain> for RawChain {
    fn from(value: Chain) -> Self {
        match value {
            Chain::Main => Self(0),
            Chain::Alt(chain_id) => Self(chain_id.0.get()),
        }
    }
}

impl From<RawChain> for Chain {
    fn from(value: RawChain) -> Self {
        NonZero::new(value.0).map_or(Self::Main, |id| Self::Alt(ChainId(id)))
    }
}

impl From<RawChainId> for RawChain {
    fn from(value: RawChainId) -> Self {
        // A [`ChainID`] with an inner value of `0` is invalid.
        assert_ne!(value.0, 0);

        Self(value.0)
    }
}

//---------------------------------------------------------------------------------------------------- RawChainId
/// [`ChainId`] in a format which can be stored in the DB.
///
/// Implements [`Into`] and [`From`] for [`ChainId`].
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// use cuprate_database::Storable;
/// use cuprate_types::ChainId;
///
/// // Assert Storable is correct.
/// let a: RawChainId = ChainId(10.try_into().unwrap()).into();
/// let b = Storable::as_bytes(&a);
/// let c: RawChainId = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_blockchain::types::*;
/// assert_eq!(size_of::<RawChainId>(), 8);
/// assert_eq!(align_of::<RawChainId>(), 8);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct RawChainId(pub(crate) u64);

impl From<ChainId> for RawChainId {
    fn from(value: ChainId) -> Self {
        Self(value.0.get())
    }
}

impl From<RawChainId> for ChainId {
    fn from(value: RawChainId) -> Self {
        Self(NonZero::new(value.0).expect("RawChainId cannot have a value of `0`"))
    }
}

//---------------------------------------------------------------------------------------------------- AltChainInfo
/// Information on an alternative chain.
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// use cuprate_database::Storable;
/// use cuprate_types::Chain;
///
/// // Assert Storable is correct.
/// let a: AltChainInfo = AltChainInfo {
///     parent_chain: Chain::Main.into(),
///     common_ancestor_height: 0,
///     chain_height: 1,
/// };
/// let b = Storable::as_bytes(&a);
/// let c: AltChainInfo = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_blockchain::types::*;
/// assert_eq!(size_of::<AltChainInfo>(), 24);
/// assert_eq!(align_of::<AltChainInfo>(), 8);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct AltChainInfo {
    /// The chain this alt chain forks from.
    pub parent_chain: RawChain,
    /// The height of the first block we share with the parent chain.
    pub common_ancestor_height: usize,
    /// The chain height of the blocks in this alt chain.
    pub chain_height: usize,
}

//---------------------------------------------------------------------------------------------------- AltBlockHeight
/// Represents the height of a block on an alt-chain.
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// use cuprate_database::Storable;
/// use cuprate_types::ChainId;
///
/// // Assert Storable is correct.
/// let a: AltBlockHeight = AltBlockHeight {
///     chain_id: ChainId(1.try_into().unwrap()).into(),
///     height: 1,
/// };
/// let b = Storable::as_bytes(&a);
/// let c: AltBlockHeight = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_blockchain::types::*;
/// assert_eq!(size_of::<AltBlockHeight>(), 16);
/// assert_eq!(align_of::<AltBlockHeight>(), 8);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct AltBlockHeight {
    /// The [`ChainId`] of the chain this alt block is on, in raw form.
    pub chain_id: RawChainId,
    /// The height of this alt-block.
    pub height: usize,
}

//---------------------------------------------------------------------------------------------------- CompactAltBlockInfo
/// Represents information on an alt-chain.
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// use cuprate_database::Storable;
///
/// // Assert Storable is correct.
/// let a: CompactAltBlockInfo = CompactAltBlockInfo {
///     block_hash: [1; 32],
///     pow_hash: [2; 32],
///     height: 10,
///     weight: 20,
///     long_term_weight: 30,
///     cumulative_difficulty_low: 40,
///     cumulative_difficulty_high: 50,
/// };
///     
/// let b = Storable::as_bytes(&a);
/// let c: CompactAltBlockInfo = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_blockchain::types::*;
/// assert_eq!(size_of::<CompactAltBlockInfo>(), 104);
/// assert_eq!(align_of::<CompactAltBlockInfo>(), 8);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct CompactAltBlockInfo {
    /// The block's hash.
    pub block_hash: [u8; 32],
    /// The block's proof-of-work hash.
    pub pow_hash: [u8; 32],
    /// The block's height.
    pub height: usize,
    /// The adjusted block size, in bytes.
    pub weight: usize,
    /// The long term block weight, which is the weight factored in with previous block weights.
    pub long_term_weight: usize,
    /// The low 64 bits of the cumulative difficulty.
    pub cumulative_difficulty_low: u64,
    /// The high 64 bits of the cumulative difficulty.
    pub cumulative_difficulty_high: u64,
}

//---------------------------------------------------------------------------------------------------- AltTransactionInfo
/// Represents information on an alt transaction.
///
/// ```rust
/// # use std::borrow::*;
/// # use cuprate_blockchain::{*, types::*};
/// use cuprate_database::Storable;
///
/// // Assert Storable is correct.
/// let a: AltTransactionInfo = AltTransactionInfo {
///     tx_weight: 1,
///     fee: 6,
///     tx_hash: [6; 32],
/// };
///     
/// let b = Storable::as_bytes(&a);
/// let c: AltTransactionInfo = Storable::from_bytes(b);
/// assert_eq!(a, c);
/// ```
///
/// # Size & Alignment
/// ```rust
/// # use cuprate_blockchain::types::*;
/// assert_eq!(size_of::<AltTransactionInfo>(), 48);
/// assert_eq!(align_of::<AltTransactionInfo>(), 8);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct AltTransactionInfo {
    /// The transaction's weight.
    pub tx_weight: usize,
    /// The transaction's total fees.
    pub fee: u64,
    /// The transaction's hash.
    pub tx_hash: [u8; 32],
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
