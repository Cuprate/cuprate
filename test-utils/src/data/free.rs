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
    BLOCK_43BD1F, BLOCK_5ECB7E, BLOCK_F91043, TX_2180A8, TX_3BC7FF, TX_84D48D, TX_9E3F73,
    TX_B6B439, TX_D7FEBD, TX_E2D393, TX_E57440,
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
    /// Turn the various static data bits in `self` into a `VerifiedBlockInformation`.
    ///
    /// Transactions are verified that they at least match the block's,
    /// although the correctness of data (whether this block actually existed or not)
    /// is not checked.
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
/// Generate a block accessor function with this signature:
///     `fn() -> &'static VerifiedBlockInformation`
///
/// This will use `VerifiedBlockMap` type above to do various
/// checks on the input data and makes sure it seems correct.
///
/// This requires some static block/tx input (from data) and some fields.
/// This data can be accessed more easily via:
/// - A block explorer (https://xmrchain.net)
/// - Monero RPC (see cuprate_test_utils::rpc for this)
///
/// See below for actual usage.
macro_rules! verified_block_information_fn {
    (
        fn_name: $fn_name:ident, // Name of the function created
        block_blob: $block_blob:ident, // Block blob ([u8], found in `constants.rs`)
        tx_blobs: [$($tx_blob:ident),*], // Array of contained transaction blobs
        pow_hash: $pow_hash:literal, // PoW hash as a string literal
        height: $height:literal, // Block height
        generated_coins: $generated_coins:literal, // Generated coins in block (`reward`)
        weight: $weight:literal, // Block weight
        long_term_weight: $long_term_weight:literal, // Block long term weight
        cumulative_difficulty: $cumulative_difficulty:literal, // Block cumulative difficulty
        tx_len: $tx_len:literal, // Amount of transactions in this block
    ) => {
        #[doc = concat!(
            "Return [`",
            stringify!($block_blob),
            "`] as a [`VerifiedBlockInformation`].",
        )]
        ///
        /// Contained transactions:
        $(
            #[doc = concat!("- [`", stringify!($tx_blob), "`]")]
        )*
        ///
        /// ```rust
        #[doc = "# use cuprate_test_utils::data::*;"]
        #[doc = "# use hex_literal::hex;"]
        #[doc = concat!("let block = ", stringify!($fn_name), "();")]
        #[doc = concat!("assert_eq!(&block.block.serialize(), ", stringify!($block_blob), ");")]
        #[doc = concat!("assert_eq!(block.pow_hash, hex!(\"", $pow_hash, "\"));")]
        #[doc = concat!("assert_eq!(block.height, ", $height, ");")]
        #[doc = concat!("assert_eq!(block.generated_coins, ", $generated_coins, ");")]
        #[doc = concat!("assert_eq!(block.weight, ", $weight, ");")]
        #[doc = concat!("assert_eq!(block.long_term_weight, ", $long_term_weight, ");")]
        #[doc = concat!("assert_eq!(block.cumulative_difficulty, ", $cumulative_difficulty, ");")]
        #[doc = concat!("assert_eq!(block.txs.len(), ", $tx_len, ");")]
        /// ```
        pub fn $fn_name() -> &'static VerifiedBlockInformation {
            static BLOCK: OnceLock<VerifiedBlockInformation> = OnceLock::new();
            BLOCK.get_or_init(|| {
                VerifiedBlockMap {
                    block: Block::read(&mut $block_blob).unwrap(),
                    pow_hash: hex!($pow_hash),
                    height: $height,
                    generated_coins: $generated_coins,
                    weight: $weight,
                    long_term_weight: $long_term_weight,
                    cumulative_difficulty: $cumulative_difficulty,
                    txs: vec![$($tx_blob),*],
                }
                .into_verified()
            })
        }
    };
}

verified_block_information_fn! {
    fn_name: block_v1_tx2,
    block_blob: BLOCK_5ECB7E,
    tx_blobs: [TX_2180A8, TX_D7FEBD],
    pow_hash: "84f64766475d51837ac9efbef1926486e58563c95a19fef4aec3254f03000000",
    height: 202_612,
    generated_coins: 13_138_270_468_431,
    weight: 55_503,
    long_term_weight: 55_503,
    cumulative_difficulty: 126_654_460_829_362,
    tx_len: 2,
}

verified_block_information_fn! {
    fn_name: block_v9_tx3,
    block_blob: BLOCK_F91043,
    tx_blobs: [TX_E2D393, TX_E57440, TX_B6B439],
    pow_hash: "7c78b5b67a112a66ea69ea51477492057dba9cfeaa2942ee7372c61800000000",
    height: 1_731_606,
    generated_coins: 3_403_921_682_163,
    weight: 6_597,
    long_term_weight: 6_597,
    cumulative_difficulty: 23_558_910_234_058_343,
    tx_len: 3,
}

verified_block_information_fn! {
    fn_name: block_v16_tx0,
    block_blob: BLOCK_43BD1F,
    tx_blobs: [],
    pow_hash: "10b473b5d097d6bfa0656616951840724dfe38c6fb9c4adf8158800300000000",
    height: 2_751_506,
    generated_coins: 600_000_000_000,
    weight: 106,
    long_term_weight: 176_470,
    cumulative_difficulty: 236_046_001_376_524_168,
    tx_len: 0,
}

//---------------------------------------------------------------------------------------------------- Transactions
/// Generate a transaction accessor function with this signature:
///     `fn() -> &'static TransactionVerificationData`
///
/// Same as [`verified_block_information_fn`] but for transactions.
macro_rules! transaction_verification_data_fn {
    (
        fn_name: $fn_name:ident, // Name of the function created
        tx_blobs: $tx_blob:ident, // Transaction blob ([u8], found in `constants.rs`)
        weight: $weight:literal, // Transaction weight
        hash: $hash:literal, // Transaction hash as a string literal
    ) => {
        #[doc = concat!("Return [`", stringify!($tx_blob), "`] as a [`TransactionVerificationData`].")]
        ///
        /// ```rust
        #[doc = "# use cuprate_test_utils::data::*;"]
        #[doc = "# use hex_literal::hex;"]
        #[doc = concat!("let tx = ", stringify!($fn_name), "();")]
        #[doc = concat!("assert_eq!(&tx.tx.serialize(), ", stringify!($tx_blob), ");")]
        #[doc = concat!("assert_eq!(tx.tx_blob, ", stringify!($tx_blob), ");")]
        #[doc = concat!("assert_eq!(tx.tx_weight, ", $weight, ");")]
        #[doc = concat!("assert_eq!(tx.tx_hash, hex!(\"", $hash, "\"));")]
        #[doc = "assert_eq!(tx.fee, tx.tx.rct_signatures.base.fee);"]
        /// ```
        pub fn $fn_name() -> &'static TransactionVerificationData {
            static TX: OnceLock<TransactionVerificationData> = OnceLock::new();
            TX.get_or_init(|| to_tx_verification_data($tx_blob))
        }
    };
}

transaction_verification_data_fn! {
    fn_name: tx_v1_sig0,
    tx_blobs: TX_3BC7FF,
    weight: 248,
    hash: "3bc7ff015b227e7313cc2e8668bfbb3f3acbee274a9c201d6211cf681b5f6bb1",
}

transaction_verification_data_fn! {
    fn_name: tx_v1_sig2,
    tx_blobs: TX_9E3F73,
    weight: 448,
    hash: "9e3f73e66d7c7293af59c59c1ff5d6aae047289f49e5884c66caaf4aea49fb34",
}

transaction_verification_data_fn! {
    fn_name: tx_v2_rct3,
    tx_blobs: TX_84D48D,
    weight: 2743,
    hash: "84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66",
}
