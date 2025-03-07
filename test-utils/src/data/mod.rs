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
//! ## Statics
//! The statics provide access to typed data found in `cuprate_types`:
//! ```rust
//! # use cuprate_test_utils::data::*;
//! use cuprate_types::{VerifiedBlockInformation, VerifiedTransactionInformation};
//!
//! let block: VerifiedBlockInformation = BLOCK_V16_TX0.clone();
//! let tx: VerifiedTransactionInformation = TX_V1_SIG0.clone();
//! ```

pub use constants::{
    BLOCK_5ECB7E, BLOCK_43BD1F, BLOCK_BBD604, BLOCK_F91043, TX_3BC7FF, TX_9E3F73, TX_84D48D,
    TX_2180A8, TX_B6B439, TX_D7FEBD, TX_E2D393, TX_E57440,
};
pub use statics::{BLOCK_V1_TX2, BLOCK_V9_TX3, BLOCK_V16_TX0, TX_V1_SIG0, TX_V1_SIG2, TX_V2_RCT3};

mod constants;
mod statics;
