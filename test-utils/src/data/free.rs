//! Free functions to access data.

#![allow(
    const_item_mutation, // `R: Read` needs `&mut self`
    clippy::missing_panics_doc, // These functions shouldn't panic
)]

//---------------------------------------------------------------------------------------------------- Import
use std::sync::OnceLock;

use monero_serai::{block::Block, transaction::Transaction};

use crate::data::constants::{
    BLOCK_43BD1F, BLOCK_BBD604, BLOCK_F91043, TX_3BC7FF, TX_84D48D, TX_9E3F73,
};

//---------------------------------------------------------------------------------------------------- Blocks
/// Return [`BLOCK_BBD604`] as a [`Block`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::block_v1_tx513().serialize(),
///     cuprate_test_utils::data::BLOCK_BBD604
/// );
/// ```
pub fn block_v1_tx513() -> Block {
    /// `OnceLock` holding the data.
    static BLOCK: OnceLock<Block> = OnceLock::new();
    BLOCK
        .get_or_init(|| Block::read(&mut BLOCK_BBD604).unwrap())
        .clone()
}

/// Return [`BLOCK_F91043`] as a [`Block`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::block_v9_tx3().serialize(),
///     cuprate_test_utils::data::BLOCK_F91043
/// );
/// ```
pub fn block_v9_tx3() -> Block {
    /// `OnceLock` holding the data.
    static BLOCK: OnceLock<Block> = OnceLock::new();
    BLOCK
        .get_or_init(|| Block::read(&mut BLOCK_F91043).unwrap())
        .clone()
}

/// Return [`BLOCK_43BD1F`] as a [`Block`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::block_v16_tx0().serialize(),
///     cuprate_test_utils::data::BLOCK_43BD1F
/// );
/// ```
pub fn block_v16_tx0() -> Block {
    /// `OnceLock` holding the data.
    static BLOCK: OnceLock<Block> = OnceLock::new();
    BLOCK
        .get_or_init(|| Block::read(&mut BLOCK_43BD1F).unwrap())
        .clone()
}

//---------------------------------------------------------------------------------------------------- Transactions
/// Return [`TX_3BC7FF`] as a [`Transaction`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::tx_v1_sig0().serialize(),
///     cuprate_test_utils::data::TX_3BC7FF
/// );
/// ```
pub fn tx_v1_sig0() -> Transaction {
    /// `OnceLock` holding the data.
    static TX: OnceLock<Transaction> = OnceLock::new();
    TX.get_or_init(|| Transaction::read(&mut TX_3BC7FF).unwrap())
        .clone()
}

/// Return [`TX_9E3F73`] as a [`Transaction`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::tx_v1_sig2().serialize(),
///     cuprate_test_utils::data::TX_9E3F73
/// );
/// ```
pub fn tx_v1_sig2() -> Transaction {
    /// `OnceLock` holding the data.
    static TX: OnceLock<Transaction> = OnceLock::new();
    TX.get_or_init(|| Transaction::read(&mut TX_9E3F73).unwrap())
        .clone()
}

/// Return [`TX_84D48D`] as a [`Transaction`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::tx_v2_rct3().serialize(),
///     cuprate_test_utils::data::TX_84D48D
/// );
/// ```
pub fn tx_v2_rct3() -> Transaction {
    /// `OnceLock` holding the data.
    static TX: OnceLock<Transaction> = OnceLock::new();
    TX.get_or_init(|| Transaction::read(&mut TX_84D48D).unwrap())
        .clone()
}
