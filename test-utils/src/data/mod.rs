//! Real Monero data.
//!
//! This module provides access to _real_ Monero data,
//! either in raw bytes or typed.
//!
//! ## Constants
//! The `const`ants provide byte slices representing block
//! and transaction blobs that can be directly deserialized:
//!
//! ```rust
//! # use cuprate_test_utils::data::*;
//! use monero_serai::{block::Block, transaction::Transaction};
//!
//! let block: Block = Block::read(&mut BLOCK_43BD1F).unwrap();
//! let tx: Transaction = Transaction::read(&mut TX_E57440).unwrap();
//! ```
//!
//! ## Functions
//! The free functions provide access to typed data found in `cuprate_types`:
//! ```rust
//! # use cuprate_test_utils::data::*;
//! use cuprate_types::{VerifiedBlockInformation, TransactionVerificationData};
//!
//! let block: VerifiedBlockInformation = block_v16_tx0().clone();
//! let tx: TransactionVerificationData = tx_v1_sig0().clone();
//! ```

mod constants;
pub use constants::{
    BLOCK_43BD1F, BLOCK_5ECB7E, BLOCK_BBD604, BLOCK_F91043, TX_2180A8, TX_3BC7FF, TX_84D48D,
    TX_9E3F73, TX_B6B439, TX_D7FEBD, TX_E2D393, TX_E57440,
};

mod free;
pub use free::{block_v16_tx0, block_v1_tx2, block_v9_tx3, tx_v1_sig0, tx_v1_sig2, tx_v2_rct3};
