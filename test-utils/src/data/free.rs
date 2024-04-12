//! Free functions to access data.

//---------------------------------------------------------------------------------------------------- Import
use std::sync::OnceLock;

use monero_serai::transaction::Transaction;

use crate::data::constants::{TX_3BC7FF, TX_84D48D, TX_9E3F73};

//---------------------------------------------------------------------------------------------------- Constants
/// Return a real `Transaction` struct.
///
/// Uses data from [`TX_3BC7FF`].
#[allow(clippy::missing_panics_doc)] // this shouldn't panic
pub fn tx_v1_sig0() -> Transaction {
    /// `OnceLock` holding the tx data.
    static TX: OnceLock<Transaction> = OnceLock::new();
    TX.get_or_init(|| {
        #[allow(const_item_mutation)] // `R: Read` needs `&mut self`
        Transaction::read(&mut TX_3BC7FF).unwrap()
    })
    .clone()
}

/// Return a real `Transaction` struct.
///
/// Uses data from [`TX_9E3F73`].
#[allow(clippy::missing_panics_doc)] // this shouldn't panic
pub fn tx_v1_sig2() -> Transaction {
    /// `OnceLock` holding the tx data.
    static TX: OnceLock<Transaction> = OnceLock::new();
    TX.get_or_init(|| {
        #[allow(const_item_mutation)] // `R: Read` needs `&mut self`
        Transaction::read(&mut TX_9E3F73).unwrap()
    })
    .clone()
}

/// Return a real `Transaction` struct.
///
/// Uses data from [`TX_84D48D`].
#[allow(clippy::missing_panics_doc)] // this shouldn't panic
pub fn tx_v2_rct3() -> Transaction {
    /// `OnceLock` holding the tx data.
    static TX: OnceLock<Transaction> = OnceLock::new();
    TX.get_or_init(|| {
        #[allow(const_item_mutation)] // `R: Read` needs `&mut self`
        Transaction::read(&mut TX_84D48D).unwrap()
    })
    .clone()
}
