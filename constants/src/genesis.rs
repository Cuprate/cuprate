//! Genesis block/transaction data.

#![expect(const_item_mutation, reason = "&mut is needed for `Read`")]

/// Generate genesis block/transaction data and tests for multiple networks.
///
/// Input string literals are in hexadecimal form.
macro_rules! generate_genesis_consts {
    ($(
        $network:ident {
            nonce: $nonce:literal,
            block: $block:literal,
            block_hash: $block_hash:literal,
            tx: $tx:literal,
            tx_hash: $tx_hash:literal,
        }
    )*) => { paste::paste! {
        $(
            #[doc = concat!(stringify!([<$network:camel>]), " data.")]
            pub mod [<$network:lower>] {
                use monero_serai::{block::Block, transaction::Transaction};
                use std::sync::LazyLock;

                #[doc = concat!("The ", stringify!([<$network:lower>]), " genesis block in [`Block`] form.")]
                pub static GENESIS_BLOCK: LazyLock<Block> =
                    LazyLock::new(|| Block::read(&mut GENESIS_BLOCK_BYTES).unwrap());

                #[doc = concat!("The ", stringify!([<$network:lower>]), " genesis block in hexadecimal form.")]
                pub const GENESIS_BLOCK_HEX: &str = $block;

                #[doc = concat!("The ", stringify!([<$network:lower>]), " genesis block in byte form.")]
                pub const GENESIS_BLOCK_BYTES: &[u8] = &hex_literal::hex!($block);

                #[doc = concat!("The hash of the ", stringify!([<$network:lower>]), " genesis block in hexadecimal form.")]
                pub const GENESIS_BLOCK_HASH_HEX: &str = $block_hash;

                #[doc = concat!("The hash of the ", stringify!([<$network:lower>]), " genesis block in byte form.")]
                pub const GENESIS_BLOCK_HASH_BYTES: [u8; 32] = hex_literal::hex!($block_hash);

                #[doc = concat!("The ", stringify!([<$network:lower>]), " genesis block in [`Transaction`] form.")]
                pub static GENESIS_TX: LazyLock<Transaction> =
                    LazyLock::new(|| Transaction::read(&mut GENESIS_TX_BYTES).unwrap());

                #[doc = concat!("The ", stringify!([<$network:lower>]), " genesis transaction in hexadecimal form.")]
                pub const GENESIS_TX_HEX: &str = $tx;

                #[doc = concat!("The ", stringify!([<$network:lower>]), " genesis transaction in byte form.")]
                pub const GENESIS_TX_BYTES: &[u8] = &hex_literal::hex!($tx);

                #[doc = concat!("The hash of the ", stringify!([<$network:lower>]), " genesis transaction in hexadecimal form.")]
                pub const GENESIS_TX_HASH_HEX: &str = $tx_hash;

                #[doc = concat!("The hash of the ", stringify!([<$network:lower>]), " genesis transaction in byte form.")]
                pub const GENESIS_TX_HASH_BYTES: [u8; 32] = hex_literal::hex!($tx_hash);

                #[doc = concat!("The nonce of the ", stringify!([<$network:lower>]), " genesis block.")]
                pub const GENESIS_NONCE: u32 = $nonce;

                // Generate tests for all input networks.
                #[cfg(test)]
                mod test {
                    use monero_serai::{block::Block, transaction::Transaction};
                    use pretty_assertions::assert_eq;

                    use super::*;

                    #[test]
                    /// Assert the block bytes/hash are correct.
                    fn genesis_block() {
                        let block = Block::read(&mut GENESIS_BLOCK_BYTES).unwrap();
                        assert_eq!(block.serialize(), &*GENESIS_BLOCK_BYTES);
                        assert_eq!(block.hash(), GENESIS_BLOCK_HASH_BYTES);
                        assert_eq!(block.hash(), GENESIS_BLOCK.hash());
                        assert_eq!(&block, &*GENESIS_BLOCK);
                    }

                    #[test]
                    /// Assert the genesis transaction in the block is correct.
                    fn genesis_block_tx() {
                        let block = Block::read(&mut GENESIS_BLOCK_BYTES).unwrap();
                        assert_eq!(block.miner_transaction.serialize(), &*GENESIS_TX_BYTES);
                        assert_eq!(block.miner_transaction.hash(), GENESIS_TX_HASH_BYTES);
                        assert_eq!(block.miner_transaction.hash(), GENESIS_BLOCK.miner_transaction.hash());
                        assert_eq!(&block.miner_transaction, &GENESIS_BLOCK.miner_transaction);
                    }

                    #[test]
                    /// Assert the hex is the same as the bytes.
                    fn genesis_block_hex_same_as_bytes() {
                        assert_eq!(
                            hex::decode(GENESIS_BLOCK_HEX).unwrap(),
                            GENESIS_BLOCK_BYTES
                        );
                        assert_eq!(
                            GENESIS_BLOCK_HEX,
                            hex::encode(GENESIS_BLOCK_BYTES)
                        );
                    }

                    #[test]
                    /// Assert the hash hex is the same as the bytes.
                    fn genesis_block_hash_hex_same_as_bytes() {
                        assert_eq!(
                            hex::decode(GENESIS_BLOCK_HASH_HEX).unwrap(),
                            GENESIS_BLOCK_HASH_BYTES
                        );
                        assert_eq!(
                            GENESIS_BLOCK_HASH_HEX,
                            hex::encode(GENESIS_BLOCK_HASH_BYTES)
                        );
                    }

                    #[test]
                    /// Assert the transaction bytes/hash are correct.
                    fn genesis_tx() {
                        let tx = Transaction::read(&mut GENESIS_TX_BYTES).unwrap();
                        assert_eq!(tx.hash(), GENESIS_TX_HASH_BYTES);
                        assert_eq!(tx.hash(), GENESIS_TX.hash());
                        assert_eq!(&tx, &*GENESIS_TX);
                    }

                    #[test]
                    /// Assert the hex is the same as the bytes.
                    fn genesis_tx_hex_same_as_bytes() {
                        assert_eq!(
                            hex::decode(GENESIS_TX_HEX).unwrap(),
                            GENESIS_TX_BYTES
                        );
                        assert_eq!(
                            GENESIS_TX_HEX,
                            hex::encode(GENESIS_TX_BYTES)
                        );
                    }

                    #[test]
                    /// Assert the hash hex is the same as the bytes.
                    fn genesis_tx_hash_hex_same_as_bytes() {
                        assert_eq!(
                            hex::decode(GENESIS_TX_HASH_HEX).unwrap(),
                            GENESIS_TX_HASH_BYTES
                        );
                        assert_eq!(
                            GENESIS_TX_HASH_HEX,
                            hex::encode(GENESIS_TX_HASH_BYTES)
                        );
                    }
                }
            }
        )*
    }};
}

generate_genesis_consts! {
    Mainnet {
        nonce: 10000,
        block: "010000000000000000000000000000000000000000000000000000000000000000000010270000013c01ff0001ffffffffffff03029b2e4c0281c0b02e7c53291a94d1d0cbff8883f8024f5142ee494ffbbd08807121017767aafcde9be00dcfd098715ebcf7f410daebc582fda69d24a28e9d0bc890d100",
        block_hash: "418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3",
        tx: "013c01ff0001ffffffffffff03029b2e4c0281c0b02e7c53291a94d1d0cbff8883f8024f5142ee494ffbbd08807121017767aafcde9be00dcfd098715ebcf7f410daebc582fda69d24a28e9d0bc890d1",
        tx_hash: "c88ce9783b4f11190d7b9c17a69c1c52200f9faaee8e98dd07e6811175177139",
    }

    Testnet {
        nonce: 10001,
        block: "010000000000000000000000000000000000000000000000000000000000000000000011270000013c01ff0001ffffffffffff03029b2e4c0281c0b02e7c53291a94d1d0cbff8883f8024f5142ee494ffbbd08807121017767aafcde9be00dcfd098715ebcf7f410daebc582fda69d24a28e9d0bc890d100",
        block_hash: "48ca7cd3c8de5b6a4d53d2861fbdaedca141553559f9be9520068053cda8430b",
        tx: "013c01ff0001ffffffffffff03029b2e4c0281c0b02e7c53291a94d1d0cbff8883f8024f5142ee494ffbbd08807121017767aafcde9be00dcfd098715ebcf7f410daebc582fda69d24a28e9d0bc890d1",
        tx_hash: "c88ce9783b4f11190d7b9c17a69c1c52200f9faaee8e98dd07e6811175177139",
    }

    Stagenet {
        nonce: 10002,
        block: "010000000000000000000000000000000000000000000000000000000000000000000012270000013c01ff0001ffffffffffff0302df5d56da0c7d643ddd1ce61901c7bdc5fb1738bfe39fbe69c28a3a7032729c0f2101168d0c4ca86fb55a4cf6a36d31431be1c53a3bd7411bb24e8832410289fa6f3b00",
        block_hash: "76ee3cc98646292206cd3e86f74d88b4dcc1d937088645e9b0cbca84b7ce74eb",
        tx: "013c01ff0001ffffffffffff0302df5d56da0c7d643ddd1ce61901c7bdc5fb1738bfe39fbe69c28a3a7032729c0f2101168d0c4ca86fb55a4cf6a36d31431be1c53a3bd7411bb24e8832410289fa6f3b",
        tx_hash: "c099809301da6ad2fde11969b0e9cb291fc698f8dc678cef00506e7baf561de4",
    }
}
