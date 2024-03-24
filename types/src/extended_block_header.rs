//! TODO

//---------------------------------------------------------------------------------------------------- Import
use bytemuck::{AnyBitPattern, NoUninit, Pod, Zeroable};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

//---------------------------------------------------------------------------------------------------- ExtendedBlockHeader
/// TODO
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Pod, Zeroable)]
#[repr(C)]
pub struct ExtendedBlockHeader {
    /// TODO
    pub version: u32, // TODO: conversion into HardFork?
    /// TODO
    pub vote: u32, // TODO: conversion into HardFork?

    // TODO: the above 2 fields are `u32` instead of
    // `u8` due to padding reasons. We could also add
    // a `[u8; 6]` here in the middle.
    /// TODO
    pub timestamp: u64,
    /// TODO
    pub cumulative_difficulty: u128,

    // TODO: The below used to be `usize` but
    // we must ensure stable layout so they
    // are set to `u64` (same since cuprate is
    // only planned for 64-bit targets?)
    /// TODO
    pub block_weight: u64,
    /// TODO
    pub long_term_weight: u64,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
