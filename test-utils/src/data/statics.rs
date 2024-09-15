//! `static LazyLock`s to access data.

#![allow(
    const_item_mutation, // `R: Read` needs `&mut self`
    clippy::missing_panics_doc, // These functions shouldn't panic
)]

//---------------------------------------------------------------------------------------------------- Import
use std::sync::LazyLock;

use hex_literal::hex;
use monero_serai::{block::Block, transaction::Transaction};

use cuprate_helper::{map::combine_low_high_bits_to_u128, tx::tx_fee};
use cuprate_types::{VerifiedBlockInformation, VerifiedTransactionInformation};

use crate::data::constants::{
    BLOCK_43BD1F, BLOCK_5ECB7E, BLOCK_F91043, TX_2180A8, TX_3BC7FF, TX_84D48D, TX_9E3F73,
    TX_B6B439, TX_D7FEBD, TX_E2D393, TX_E57440,
};

//---------------------------------------------------------------------------------------------------- Conversion
/// Converts [`monero_serai::Block`] into a
/// [`VerifiedBlockInformation`] (superset).
///
/// To prevent pulling other code in order to actually calculate things
/// (e.g. `pow_hash`), some information must be provided statically,
/// this struct represents that data that must be provided.
///
/// Consider using [`cuprate_test_utils::rpc`] to get this data easily.
struct VerifiedBlockMap {
    block_blob: &'static [u8],
    pow_hash: [u8; 32],
    height: usize,
    generated_coins: u64,
    weight: usize,
    long_term_weight: usize,
    cumulative_difficulty_low: u64,
    cumulative_difficulty_high: u64,
    // Vec of `tx_blob`'s, i.e. the data in `/test-utils/src/data/tx/`.
    // This should the actual `tx_blob`'s of the transactions within this block.
    txs: &'static [&'static [u8]],
}

impl VerifiedBlockMap {
    /// Turn the various static data bits in `self` into a [`VerifiedBlockInformation`].
    ///
    /// Transactions are verified that they at least match the block's,
    /// although the correctness of data (whether this block actually existed or not)
    /// is not checked.
    fn into_verified(self) -> VerifiedBlockInformation {
        let Self {
            block_blob,
            pow_hash,
            height,
            generated_coins,
            weight,
            long_term_weight,
            cumulative_difficulty_low,
            cumulative_difficulty_high,
            txs,
        } = self;

        let block_blob = block_blob.to_vec();
        let block = Block::read(&mut block_blob.as_slice()).unwrap();

        let txs = txs.iter().map(to_tx_verification_data).collect::<Vec<_>>();

        assert_eq!(
            txs.len(),
            block.transactions.len(),
            "(deserialized txs).len() != (txs hashes in block).len()"
        );

        for (tx, tx_hash_in_block) in txs.iter().zip(&block.transactions) {
            assert_eq!(
                &tx.tx_hash, tx_hash_in_block,
                "deserialized tx hash is not the same as the one in the parent block"
            );
        }

        VerifiedBlockInformation {
            block_hash: block.hash(),
            block_blob,
            block,
            txs,
            pow_hash,
            height,
            generated_coins,
            weight,
            long_term_weight,
            cumulative_difficulty: combine_low_high_bits_to_u128(
                cumulative_difficulty_low,
                cumulative_difficulty_high,
            ),
        }
    }
}

// Same as [`VerifiedBlockMap`] but for [`VerifiedTransactionInformation`].
fn to_tx_verification_data(tx_blob: impl AsRef<[u8]>) -> VerifiedTransactionInformation {
    let tx_blob = tx_blob.as_ref().to_vec();
    let tx = Transaction::read(&mut tx_blob.as_slice()).unwrap();
    VerifiedTransactionInformation {
        tx_weight: tx.weight(),
        fee: tx_fee(&tx),
        tx_hash: tx.hash(),
        tx_blob,
        tx,
    }
}

//---------------------------------------------------------------------------------------------------- Blocks
/// Generate a `static LazyLock<VerifiedBlockInformation>`.
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
macro_rules! verified_block_information {
    (
        name: $name:ident, // Name of the `LazyLock` created
        block_blob: $block_blob:ident, // Block blob ([u8], found in `constants.rs`)
        tx_blobs: [$($tx_blob:ident),*], // Array of contained transaction blobs
        pow_hash: $pow_hash:literal, // PoW hash as a string literal
        height: $height:literal, // Block height
        generated_coins: $generated_coins:literal, // Generated coins in block (minus fees)
        weight: $weight:literal, // Block weight
        long_term_weight: $long_term_weight:literal, // Block long term weight
        cumulative_difficulty_low: $cumulative_difficulty_low:literal, // Least significant 64-bits of block cumulative difficulty
        cumulative_difficulty_high: $cumulative_difficulty_high:literal, // Most significant 64-bits of block cumulative difficulty
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
        #[doc = "use cuprate_helper::map::combine_low_high_bits_to_u128;"]
        #[doc = ""]
        #[doc = concat!("let block = &*", stringify!($name), ";")]
        #[doc = concat!("assert_eq!(&block.block.serialize(), ", stringify!($block_blob), ");")]
        #[doc = concat!("assert_eq!(block.pow_hash, hex!(\"", $pow_hash, "\"));")]
        #[doc = concat!("assert_eq!(block.height, ", $height, ");")]
        #[doc = concat!("assert_eq!(block.generated_coins, ", $generated_coins, ");")]
        #[doc = concat!("assert_eq!(block.weight, ", $weight, ");")]
        #[doc = concat!("assert_eq!(block.long_term_weight, ", $long_term_weight, ");")]
        #[doc = concat!("assert_eq!(block.txs.len(), ", $tx_len, ");")]
        #[doc = ""]
        #[doc = concat!(
            "assert_eq!(block.cumulative_difficulty, ",
            "combine_low_high_bits_to_u128(",
            stringify!($cumulative_difficulty_low),
            ", ",
            stringify!($cumulative_difficulty_high),
            "));"
        )]
        /// ```
        pub static $name: LazyLock<VerifiedBlockInformation> = LazyLock::new(|| {
            VerifiedBlockMap {
                block_blob: $block_blob,
                pow_hash: hex!($pow_hash),
                height: $height,
                generated_coins: $generated_coins,
                weight: $weight,
                long_term_weight: $long_term_weight,
                cumulative_difficulty_low: $cumulative_difficulty_low,
                cumulative_difficulty_high: $cumulative_difficulty_high,
                txs: &[$($tx_blob),*],
            }
            .into_verified()
        });
    };
}

verified_block_information! {
    name: BLOCK_V1_TX2,
    block_blob: BLOCK_5ECB7E,
    tx_blobs: [TX_2180A8, TX_D7FEBD],
    pow_hash: "c960d540000459480560b7816de968c7470083e5874e10040bdd4cc501000000",
    height: 202_609,
    generated_coins: 14_535_350_982_449,
    weight: 21_905,
    long_term_weight: 21_905,
    cumulative_difficulty_low: 126_650_740_038_710,
    cumulative_difficulty_high: 0,
    tx_len: 2,
}

verified_block_information! {
    name: BLOCK_V9_TX3,
    block_blob: BLOCK_F91043,
    tx_blobs: [TX_E2D393, TX_E57440, TX_B6B439],
    pow_hash: "7c78b5b67a112a66ea69ea51477492057dba9cfeaa2942ee7372c61800000000",
    height: 1_731_606,
    generated_coins: 3_403_774_022_163,
    weight: 6_597,
    long_term_weight: 6_597,
    cumulative_difficulty_low: 23_558_910_234_058_343,
    cumulative_difficulty_high: 0,
    tx_len: 3,
}

verified_block_information! {
    name: BLOCK_V16_TX0,
    block_blob: BLOCK_43BD1F,
    tx_blobs: [],
    pow_hash: "10b473b5d097d6bfa0656616951840724dfe38c6fb9c4adf8158800300000000",
    height: 2_751_506,
    generated_coins: 600_000_000_000,
    weight: 106,
    long_term_weight: 176_470,
    cumulative_difficulty_low: 236_046_001_376_524_168,
    cumulative_difficulty_high: 0,
    tx_len: 0,
}

//---------------------------------------------------------------------------------------------------- Transactions
/// Generate a `const LazyLock<VerifiedTransactionInformation>`.
///
/// Same as [`verified_block_information`] but for transactions.
macro_rules! transaction_verification_data {
    (
        name: $name:ident, // Name of the `LazyLock` created
        tx_blobs: $tx_blob:ident, // Transaction blob ([u8], found in `constants.rs`)
        weight: $weight:literal, // Transaction weight
        hash: $hash:literal, // Transaction hash as a string literal
    ) => {
        #[doc = concat!("Return [`", stringify!($tx_blob), "`] as a [`VerifiedTransactionInformation`].")]
        ///
        /// ```rust
        #[doc = "# use cuprate_test_utils::data::*;"]
        #[doc = "# use hex_literal::hex;"]
        #[doc = concat!("let tx = &*", stringify!($name), ";")]
        #[doc = concat!("assert_eq!(&tx.tx.serialize(), ", stringify!($tx_blob), ");")]
        #[doc = concat!("assert_eq!(tx.tx_blob, ", stringify!($tx_blob), ");")]
        #[doc = concat!("assert_eq!(tx.tx_weight, ", $weight, ");")]
        #[doc = concat!("assert_eq!(tx.tx_hash, hex!(\"", $hash, "\"));")]
        /// ```
        pub static $name: LazyLock<VerifiedTransactionInformation> = LazyLock::new(|| {
            to_tx_verification_data($tx_blob)
        });
    };
}

transaction_verification_data! {
    name: TX_V1_SIG0,
    tx_blobs: TX_3BC7FF,
    weight: 248,
    hash: "3bc7ff015b227e7313cc2e8668bfbb3f3acbee274a9c201d6211cf681b5f6bb1",
}

transaction_verification_data! {
    name: TX_V1_SIG2,
    tx_blobs: TX_9E3F73,
    weight: 448,
    hash: "9e3f73e66d7c7293af59c59c1ff5d6aae047289f49e5884c66caaf4aea49fb34",
}

transaction_verification_data! {
    name: TX_V2_RCT3,
    tx_blobs: TX_84D48D,
    weight: 2743,
    hash: "84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66",
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::rpc::client::HttpRpcClient;

    use super::*;

    /// Assert the defined blocks are the same compared to ones received from a local RPC call.
    #[ignore] // FIXME: doesn't work in CI, we need a real unrestricted node
    #[tokio::test]
    async fn block_same_as_rpc() {
        let rpc = HttpRpcClient::new(None).await;
        for block in [&*BLOCK_V1_TX2, &*BLOCK_V9_TX3, &*BLOCK_V16_TX0] {
            println!("block_height: {}", block.height);
            let block_rpc = rpc.get_verified_block_information(block.height).await;
            assert_eq!(block, &block_rpc);
        }
    }

    /// Same as `block_same_as_rpc` but for transactions.
    /// This also tests all the transactions within the defined blocks.
    #[ignore] // FIXME: doesn't work in CI, we need a real unrestricted node
    #[tokio::test]
    async fn tx_same_as_rpc() {
        let rpc = HttpRpcClient::new(None).await;

        let mut txs = [&*BLOCK_V1_TX2, &*BLOCK_V9_TX3, &*BLOCK_V16_TX0]
            .into_iter()
            .flat_map(|block| block.txs.iter().cloned())
            .collect::<Vec<VerifiedTransactionInformation>>();

        txs.extend([TX_V1_SIG0.clone(), TX_V1_SIG2.clone(), TX_V2_RCT3.clone()]);

        for tx in txs {
            println!("tx_hash: {:?}", tx.tx_hash);
            let tx_rpc = rpc
                .get_transaction_verification_data(&[tx.tx_hash])
                .await
                .collect::<Vec<VerifiedTransactionInformation>>()
                .pop()
                .unwrap();
            assert_eq!(tx, tx_rpc);
        }
    }
}
