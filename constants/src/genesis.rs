//! Genesis block/transaction data.

#![allow(const_item_mutation, reason = "&mut is needed for `Read`")]

#[cfg(feature = "monero-serai")]
use monero_serai::{block::Block, transaction::Transaction};
#[cfg(feature = "monero-serai")]
use std::sync::LazyLock;

/// The genesis block in [`Block`] form.
#[cfg(feature = "monero-serai")]
pub static GENESIS_BLOCK: LazyLock<Block> =
    LazyLock::new(|| Block::read(&mut GENESIS_BLOCK_BYTES).unwrap());

/// The genesis block in hexadecimal form.
pub const GENESIS_BLOCK_HEX: &str = "010000000000000000000000000000000000000000000000000000000000000000000010270000013c01ff0001ffffffffffff03029b2e4c0281c0b02e7c53291a94d1d0cbff8883f8024f5142ee494ffbbd08807121017767aafcde9be00dcfd098715ebcf7f410daebc582fda69d24a28e9d0bc890d100";

/// The genesis block in byte form.
pub const GENESIS_BLOCK_BYTES: &[u8] = &[
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 16, 39, 0, 0, 1, 60, 1, 255, 0, 1, 255, 255, 255, 255, 255, 255, 3, 2, 155, 46, 76, 2,
    129, 192, 176, 46, 124, 83, 41, 26, 148, 209, 208, 203, 255, 136, 131, 248, 2, 79, 81, 66, 238,
    73, 79, 251, 189, 8, 128, 113, 33, 1, 119, 103, 170, 252, 222, 155, 224, 13, 207, 208, 152,
    113, 94, 188, 247, 244, 16, 218, 235, 197, 130, 253, 166, 157, 36, 162, 142, 157, 11, 200, 144,
    209, 0,
];

/// The hash of the genesis block in hexadecimal form.
pub const GENESIS_BLOCK_HASH_HEX: &str =
    "418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3";

/// The hash of the genesis block in byte form.
pub const GENESIS_BLOCK_HASH_BYTES: [u8; 32] = [
    65, 128, 21, 187, 154, 233, 130, 161, 151, 93, 167, 215, 146, 119, 194, 112, 87, 39, 165, 104,
    148, 186, 15, 178, 70, 173, 170, 187, 31, 70, 50, 227,
];

/// The genesis block in [`Transaction`] form.
#[cfg(feature = "monero-serai")]
pub static GENESIS_TX: LazyLock<Transaction> =
    LazyLock::new(|| Transaction::read(&mut GENESIS_TX_BYTES).unwrap());

/// The genesis transaction in hexadecimal form.
pub const GENESIS_TX_HEX: &str = "013c01ff0001ffffffffffff03029b2e4c0281c0b02e7c53291a94d1d0cbff8883f8024f5142ee494ffbbd08807121017767aafcde9be00dcfd098715ebcf7f410daebc582fda69d24a28e9d0bc890d1";

/// The genesis transaction in byte form.
pub const GENESIS_TX_BYTES: &[u8] = &[
    1, 60, 1, 255, 0, 1, 255, 255, 255, 255, 255, 255, 3, 2, 155, 46, 76, 2, 129, 192, 176, 46,
    124, 83, 41, 26, 148, 209, 208, 203, 255, 136, 131, 248, 2, 79, 81, 66, 238, 73, 79, 251, 189,
    8, 128, 113, 33, 1, 119, 103, 170, 252, 222, 155, 224, 13, 207, 208, 152, 113, 94, 188, 247,
    244, 16, 218, 235, 197, 130, 253, 166, 157, 36, 162, 142, 157, 11, 200, 144, 209,
];

/// The hash of the genesis transaction in hexadecimal form.
pub const GENESIS_TX_HASH_HEX: &str =
    "c88ce9783b4f11190d7b9c17a69c1c52200f9faaee8e98dd07e6811175177139";

/// The hash of the genesis transaction in byte form.
pub const GENESIS_TX_HASH_BYTES: [u8; 32] = [
    200, 140, 233, 120, 59, 79, 17, 25, 13, 123, 156, 23, 166, 156, 28, 82, 32, 15, 159, 170, 238,
    142, 152, 221, 7, 230, 129, 17, 117, 23, 113, 57,
];

#[cfg(test)]
mod test {
    use monero_serai::{block::Block, transaction::Transaction};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn genesis_block() {
        assert_eq!(hex::decode(GENESIS_BLOCK_HEX).unwrap(), GENESIS_BLOCK_BYTES);
        assert_eq!(GENESIS_BLOCK_HEX, hex::encode(GENESIS_BLOCK_BYTES));

        assert_eq!(
            hex::decode(GENESIS_BLOCK_HASH_HEX).unwrap(),
            GENESIS_BLOCK_HASH_BYTES
        );
        assert_eq!(
            GENESIS_BLOCK_HASH_HEX,
            hex::encode(GENESIS_BLOCK_HASH_BYTES)
        );

        let block = Block::read(&mut GENESIS_BLOCK_BYTES).unwrap();
        assert_eq!(block.hash(), GENESIS_BLOCK_HASH_BYTES);
        assert_eq!(block.hash(), GENESIS_BLOCK.hash());
        assert_eq!(&block, &*GENESIS_BLOCK);
    }

    #[test]
    fn genesis_tx() {
        assert_eq!(hex::decode(GENESIS_TX_HEX).unwrap(), GENESIS_TX_BYTES);
        assert_eq!(GENESIS_TX_HEX, hex::encode(GENESIS_TX_BYTES));

        assert_eq!(
            hex::decode(GENESIS_TX_HASH_HEX).unwrap(),
            GENESIS_TX_HASH_BYTES
        );
        assert_eq!(GENESIS_TX_HASH_HEX, hex::encode(GENESIS_TX_HASH_BYTES));

        let tx = Transaction::read(&mut GENESIS_TX_BYTES).unwrap();
        assert_eq!(tx.hash(), GENESIS_TX_HASH_BYTES);
        assert_eq!(tx.hash(), GENESIS_TX.hash());
        assert_eq!(&tx, &*GENESIS_TX);
    }
}
