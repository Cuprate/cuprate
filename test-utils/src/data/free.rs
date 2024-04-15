//! Free functions to access data.

#![allow(
    const_item_mutation, // `R: Read` needs `&mut self`
    clippy::missing_panics_doc, // These functions shouldn't panic
)]

//---------------------------------------------------------------------------------------------------- Import
use std::sync::{Arc, OnceLock};

use hex_literal::hex;
use monero_serai::{block::Block, transaction::Transaction};

use cuprate_types::{TransactionVerificationData, VerifiedBlockInformation};

use crate::data::constants::{
    BLOCK_43BD1F, BLOCK_BBD604, BLOCK_F91043, TX_3BC7FF, TX_84D48D, TX_9E3F73,
};

//---------------------------------------------------------------------------------------------------- Conversion
/// FIXME: this isn't ideal, create a way to do this automatically
/// and to actually verify the static data provided is correct.
///
/// Converts `monero_serai`'s `Block` into a
/// `cuprate_types::VerifiedBlockInformation` (superset).
///
/// To prevent pulling other code in order to actually calculate things
/// (e.g. `pow_hash`), some information must be provided statically.
struct VerifiedBlockMap {
    block: Block,
    pow_hash: [u8; 32],
    height: u64,
    generated_coins: u64,
    weight: usize,
    long_term_weight: usize,
    cumulative_difficulty: u128,
    // .len() should be == `block.tx.len()` and should contain the
    // fees for each transaction, i.e. `block.tx[0]`s fee is `tx_fees[0]`.
    tx_fees: Vec<u64>,
}

impl VerifiedBlockMap {
    fn into_verified(self) -> VerifiedBlockInformation {
        let Self {
            block,
            pow_hash,
            height,
            generated_coins,
            weight,
            long_term_weight,
            cumulative_difficulty,
            tx_fees,
        } = self;

        let txs: Vec<Arc<TransactionVerificationData>> = block
            .txs
            .clone()
            .iter()
            .enumerate()
            .map(|(i, tx_blob)| {
                VerifiedTxMap {
                    tx_blob: tx_blob.to_vec(),
                    fee: tx_fees[i],
                }
                .into_verified()
            })
            .map(Arc::new)
            .collect();

        VerifiedBlockInformation {
            block_hash: block.hash(),
            block,
            txs,
            pow_hash,
            height,
            generated_coins,
            weight,
            long_term_weight,
            cumulative_difficulty,
        }
    }
}

// Same as [`VerifiedBlockMap`] but for [`TransactionVerificationData`].
struct VerifiedTxMap {
    tx_blob: Vec<u8>,
    fee: u64,
}

impl VerifiedTxMap {
    fn into_verified(self) -> TransactionVerificationData {
        let tx = Transaction::read(&mut self.tx_blob.as_slice()).unwrap();
        TransactionVerificationData {
            tx_blob: self.tx_blob,
            tx_weight: tx.weight(),
            fee: self.fee,
            tx_hash: tx.hash(),
            tx,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Blocks
/// TODO: create a macro to generate these functions.

/// Return [`BLOCK_BBD604`] as a [`VerifiedBlockInformation`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::block_v1_tx513().block.serialize(),
///     cuprate_test_utils::data::BLOCK_BBD604
/// );
/// ```
pub fn block_v1_tx513() -> &'static VerifiedBlockInformation {
    /// `OnceLock` holding the data.
    static BLOCK: OnceLock<VerifiedBlockInformation> = OnceLock::new();
    BLOCK.get_or_init(|| {
        VerifiedBlockMap {
            block: Block::read(&mut BLOCK_BBD604).unwrap(),
            pow_hash: hex!("84f64766475d51837ac9efbef1926486e58563c95a19fef4aec3254f03000000"),
            height: 202_612,
            generated_coins: 13_138_270_468_431,
            weight: 55_503,
            long_term_weight: 55_503,
            cumulative_difficulty: 126_654_460_829_362,
            tx_fees: vec![0; 513], // 513 tx's, 0 fees
        }
        .into_verified()
    })
}

/// Return [`BLOCK_F91043`] as a [`VerifiedBlockInformation`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::block_v9_tx3().block.serialize(),
///     cuprate_test_utils::data::BLOCK_F91043
/// );
/// ```
pub fn block_v9_tx3() -> &'static VerifiedBlockInformation {
    /// `OnceLock` holding the data.
    static BLOCK: OnceLock<VerifiedBlockInformation> = OnceLock::new();
    BLOCK.get_or_init(|| {
        VerifiedBlockMap {
            block: Block::read(&mut BLOCK_F91043).unwrap(),
            pow_hash: hex!("7c78b5b67a112a66ea69ea51477492057dba9cfeaa2942ee7372c61800000000"),
            height: 1_731_606,
            generated_coins: 3_403_921_682_163,
            weight: 6_597,
            long_term_weight: 6_597,
            cumulative_difficulty: 23_558_910_234_058_343,
            tx_fees: vec![43370000, 42820000, 61470000], // 3 tx's
        }
        .into_verified()
    })
}

/// Return [`BLOCK_43BD1F`] as a [`VerifiedBlockInformation`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::block_v16_tx0().block.serialize(),
///     cuprate_test_utils::data::BLOCK_43BD1F
/// );
/// ```
pub fn block_v16_tx0() -> &'static VerifiedBlockInformation {
    /// `OnceLock` holding the data.
    static BLOCK: OnceLock<VerifiedBlockInformation> = OnceLock::new();
    BLOCK.get_or_init(|| {
        VerifiedBlockMap {
            block: Block::read(&mut BLOCK_43BD1F).unwrap(),
            pow_hash: hex!("10b473b5d097d6bfa0656616951840724dfe38c6fb9c4adf8158800300000000"),
            height: 2_751_506,
            generated_coins: 600_000_000_000,
            weight: 106,
            long_term_weight: 176_470,
            cumulative_difficulty: 236_046_001_376_524_168,
            tx_fees: vec![], // 0 tx's, 0 fees
        }
        .into_verified()
    })
}

//---------------------------------------------------------------------------------------------------- Transactions
/// Return [`TX_3BC7FF`] as a [`TransactionVerificationData`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::tx_v1_sig0().tx.serialize(),
///     cuprate_test_utils::data::TX_3BC7FF
/// );
/// ```
pub fn tx_v1_sig0() -> &'static TransactionVerificationData {
    /// `OnceLock` holding the data.
    static TX: OnceLock<TransactionVerificationData> = OnceLock::new();
    TX.get_or_init(|| {
        VerifiedTxMap {
            tx_blob: TX_3BC7FF.to_vec(),
            fee: 0,
        }
        .into_verified()
    })
}

/// Return [`TX_9E3F73`] as a [`TransactionVerificationData`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::tx_v1_sig2().tx.serialize(),
///     cuprate_test_utils::data::TX_9E3F73
/// );
/// ```
pub fn tx_v1_sig2() -> &'static TransactionVerificationData {
    /// `OnceLock` holding the data.
    static TX: OnceLock<TransactionVerificationData> = OnceLock::new();
    TX.get_or_init(|| {
        VerifiedTxMap {
            tx_blob: TX_9E3F73.to_vec(),
            fee: 14_000_000_000,
        }
        .into_verified()
    })
}

/// Return [`TX_84D48D`] as a [`TransactionVerificationData`].
///
/// ```rust
/// assert_eq!(
///     &cuprate_test_utils::data::tx_v2_rct3().tx.serialize(),
///     cuprate_test_utils::data::TX_84D48D
/// );
/// ```
pub fn tx_v2_rct3() -> &'static TransactionVerificationData {
    /// `OnceLock` holding the data.
    static TX: OnceLock<TransactionVerificationData> = OnceLock::new();
    TX.get_or_init(|| {
        VerifiedTxMap {
            tx_blob: TX_84D48D.to_vec(),
            fee: 1_401_270_000,
        }
        .into_verified()
    })
}
