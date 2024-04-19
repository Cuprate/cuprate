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
/// Converts `monero_serai`'s `Block` into a
/// `cuprate_types::VerifiedBlockInformation` (superset).
///
/// To prevent pulling other code in order to actually calculate things
/// (e.g. `pow_hash`), some information must be provided statically,
/// this struct represents that data that must be provided.
///
/// Consider using `cuprate_test_utils::rpc` to get this data easily.
struct VerifiedBlockMap<'a> {
    block: Block,
    pow_hash: [u8; 32],
    height: u64,
    generated_coins: u64,
    weight: usize,
    long_term_weight: usize,
    cumulative_difficulty: u128,
    // Vec of `tx_blob`'s, i.e. the data in `/test-utils/src/data/tx/`.
    // This should the actual `tx_blob`'s of the transactions within this block.
    txs: Vec<&'a [u8]>,
}

impl VerifiedBlockMap<'_> {
    fn into_verified(self) -> VerifiedBlockInformation {
        let Self {
            block,
            pow_hash,
            height,
            generated_coins,
            weight,
            long_term_weight,
            cumulative_difficulty,
            txs,
        } = self;

        let txs: Vec<Arc<TransactionVerificationData>> = txs
            .into_iter()
            .map(to_tx_verification_data)
            .map(Arc::new)
            .collect();

        assert_eq!(
            txs.len(),
            block.txs.len(),
            "(deserialized txs).len() != (txs hashes in block).len()"
        );

        for (tx, tx_hash_in_block) in txs.iter().zip(&block.txs) {
            assert_eq!(
                &tx.tx_hash, tx_hash_in_block,
                "deserialized tx hash is not the same as the one in the parent block"
            );
        }

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
fn to_tx_verification_data(tx_blob: &[u8]) -> TransactionVerificationData {
    let tx_blob = tx_blob.to_vec();
    let tx = Transaction::read(&mut tx_blob.as_slice()).unwrap();
    TransactionVerificationData {
        tx_weight: tx.weight(),
        fee: tx.rct_signatures.base.fee,
        tx_hash: tx.hash(),
        tx_blob,
        tx,
    }
}

//---------------------------------------------------------------------------------------------------- Blocks
/// TODO: create a macro to generate these functions.

/// Block with height `202611` and hash `5da0a3d004c352a90cc86b00fab676695d76a4d1de16036c41ba4dd188c4d76f`.
///
/// ```rust
/// use monero_serai::{block::Block, transaction::Input};
///
/// let block = block_v1_tx1();
///
/// assert_eq!(block.block.header.major_version, 1);
/// assert_eq!(block.block.header.minor_version, 0);
/// assert_eq!(block.block.header.timestamp, 1409804537);
/// assert_eq!(block.block.header.nonce, 481);
/// assert!(matches!(block.block.miner_tx.prefix.inputs[0], Input::Gen(202612)));
/// assert_eq!(block.block.txs.len(), 3);
/// assert_eq!(
///     hex::encode(block.block.hash()),
///     "5da0a3d004c352a90cc86b00fab676695d76a4d1de16036c41ba4dd188c4d76f",
/// );
///
///
/// ```
pub fn block_v1_tx513() -> &'static VerifiedBlockInformation {
    const BLOCK_BLOB: &[u8] = include_bytes!(
        "block/5da0a3d004c352a90cc86b00fab676695d76a4d1de16036c41ba4dd188c4d76f.bin"
    );
    static BLOCK: OnceLock<VerifiedBlockInformation> = OnceLock::new();
    BLOCK.get_or_init(|| {
        VerifiedBlockMap {
            block: Block::read(&mut BLOCK_BLOB).unwrap(),
            pow_hash: hex!("84f64766475d51837ac9efbef1926486e58563c95a19fef4aec3254f03000000"),
            height: 202_612,
            generated_coins: 13_138_270_468_431,
            weight: 55_503,
            long_term_weight: 55_503,
            cumulative_difficulty: 126_654_460_829_362,
            txs: vec![],
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
            txs: vec![],
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
            txs: vec![],
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
    TX.get_or_init(|| to_tx_verification_data(TX_3BC7FF))
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
    TX.get_or_init(|| to_tx_verification_data(TX_9E3F73))
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
    TX.get_or_init(|| to_tx_verification_data(TX_84D48D))
}
