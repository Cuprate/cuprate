//! Database [table](crate::tables) types.
//!
//! This module contains all types used by the database tables.
//!
//! TODO: Add schema here too.

/*
 * <============================================> VERY BIG SCARY SAFETY MESSAGE <============================================>
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 *
 *
 *
 * We use `bytemuck` to (de)serialize data types in the database.
 * We are UNSAFELY casting bytes, and as such, we must uphold some invariants.
 * When editing this file, there is only 1 commandment that MUST be followed:
 *
 *   1. Thou shall only implement `bytemuck` traits using the derive macros
 *
 * The derive macros will fail at COMPILE time if something is incorrect.
 * <https://docs.rs/bytemuck/latest/bytemuck/derive.Pod.html>
 * If you submit a PR that breaks this I will come and find you.
 *
 *
 *
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE --- DO NOT IGNORE
 * <============================================> VERY BIG SCARY SAFETY MESSAGE <============================================>
 */
// actually i still don't trust you. no unsafe.
#![forbid(unsafe_code)] // if you remove this line i will steal your monero
#![allow(missing_docs)] // bytemuck auto-generates some non-documented structs

//---------------------------------------------------------------------------------------------------- Import
use bytemuck::{CheckedBitPattern, NoUninit, Pod, Zeroable};

#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

//---------------------------------------------------------------------------------------------------- TestType
/// ```rust
/// # use cuprate_database::types::*;
/// let a = TestType { u: 1, b: true, _pad: [0; 7] }; // original struct
/// let b = bytemuck::must_cast::<TestType, [u8; 16]>(a); // cast into bytes
/// let c = bytemuck::checked::cast::<[u8; 16], TestType>(b); // cast back into struct
///
/// assert_eq!(a, c);
/// assert_eq!(c.u, 1);
/// assert_eq!(c.b, true);
/// assert_eq!(c._pad, [0; 7]);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, NoUninit, CheckedBitPattern)]
#[repr(C)]
pub struct TestType {
    /// TEST
    pub u: usize,
    /// TEST
    pub b: bool,
    /// TEST
    pub _pad: [u8; 7],
}

//---------------------------------------------------------------------------------------------------- TestType2
/// ```rust
/// # use cuprate_database::types::*;
/// let a = TestType2 { u: 1, b: [1; 32] }; // original struct
/// let b = bytemuck::must_cast::<TestType2, [u8; 40]>(a); // cast into bytes
/// let c = bytemuck::must_cast::<[u8; 40], TestType2>(b); // cast back into struct
///
/// assert_eq!(a, c);
/// assert_eq!(c.u, 1);
/// assert_eq!(c.b, [1; 32]);
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct TestType2 {
    /// TEST
    pub u: usize,
    /// TEST
    pub b: [u8; 32],
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
