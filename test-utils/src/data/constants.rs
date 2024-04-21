//! Constants holding raw Monero data.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Block
/// Generate a `const _: &[u8]` pointing to a block blob.
///
/// This will deserialize with `Block` to assume the blob is at least deserializable.
///
/// This requires some static block input for testing.
///
/// The actual block blob data on disk is found in `data/block`.
///
/// See below for actual usage.
macro_rules! const_block_blob {
    (
        name: $name:ident, // Name of the `const` created
        height: $height:literal, // Block height
        hash: $hash:literal, // Block hash
        data_path: $data_path:literal, // Path to the block blob
        major_version: $major_version:literal, // Block's major version
        minor_version: $minor_version:literal, // Block's minor version
        timestamp: $timestamp:literal, // Block's timestamp
        nonce: $nonce:literal, // Block's nonce
        miner_tx_generated: $miner_tx_generated:literal, // Generated Monero in block's miner transaction
        tx_len: $tx_len:literal, // How many transactions there are in the block
    ) => {
        #[doc = concat!("Block with hash `", $hash, "`.")]
        ///
        #[doc = concat!("Height: `", $height, "`.")]
        ///
        /// ```rust
        #[doc = "# use cuprate_test_utils::data::*;"]
        #[doc = "use monero_serai::{block::Block, transaction::Input};"]
        #[doc = ""]
        #[doc = concat!("let block = Block::read(&mut ", stringify!($name), ").unwrap();")]
        #[doc = ""]
        #[doc = concat!("assert_eq!(block.header.major_version, ", $major_version, ");")]
        #[doc = concat!("assert_eq!(block.header.minor_version, ", $minor_version, ");")]
        #[doc = concat!("assert_eq!(block.header.timestamp, ", $timestamp, ");")]
        #[doc = concat!("assert_eq!(block.header.nonce, ", $nonce, ");")]
        #[doc = concat!("assert!(matches!(block.miner_tx.prefix.inputs[0], Input::Gen(", $miner_tx_generated, ")));")]
        #[doc = concat!("assert_eq!(block.txs.len(), ", $tx_len, ");")]
        #[doc = concat!("assert_eq!(hex::encode(block.hash()), \"", $hash, "\")")]
        /// ```
        pub const $name: &[u8] = include_bytes!($data_path);
    };
}

const_block_blob! {
    name: BLOCK_BBD604,
    height: 202_612,
    hash: "bbd604d2ba11ba27935e006ed39c9bfdd99b76bf4a50654bc1e1e61217962698",
    data_path: "block/bbd604d2ba11ba27935e006ed39c9bfdd99b76bf4a50654bc1e1e61217962698.bin",
    major_version: 1,
    minor_version: 0,
    timestamp: 1409804570,
    nonce: 1073744198,
    miner_tx_generated: 202612,
    tx_len: 513,
}

const_block_blob! {
    name: BLOCK_5ECB7E,
    height: 202_609,
    hash: "5ecb7e663bbe947c734c8059e7d7d52dc7d6644bb82d81a6ad4057d127ee8eda",
    data_path: "block/5ecb7e663bbe947c734c8059e7d7d52dc7d6644bb82d81a6ad4057d127ee8eda.bin",
    major_version: 1,
    minor_version: 0,
    timestamp: 1409804315,
    nonce: 48426,
    miner_tx_generated: 202609,
    tx_len: 2,
}

const_block_blob! {
    name: BLOCK_F91043,
    height: 2_751_506,
    hash: "f910435a5477ca27be1986c080d5476aeab52d0c07cf3d9c72513213350d25d4",
    data_path: "block/f910435a5477ca27be1986c080d5476aeab52d0c07cf3d9c72513213350d25d4.bin",
    major_version: 9,
    minor_version: 9,
    timestamp: 1545423190,
    nonce: 4123173351,
    miner_tx_generated: 1731606,
    tx_len: 3,
}

const_block_blob! {
    name: BLOCK_43BD1F,
    height: 2_751_506,
    hash: "43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428",
    data_path: "block/43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428.bin",
    major_version: 16,
    minor_version: 16,
    timestamp: 1667941829,
    nonce: 4110909056,
    miner_tx_generated: 2751506,
    tx_len: 0,
}

//---------------------------------------------------------------------------------------------------- Transaction
/// Generate a `const _: &[u8]` pointing to a transaction blob.
///
/// Same as [`const_block_blob`] but for transactions.
macro_rules! const_tx_blob {
    (
        name: $name:ident, // Name of the `const` created
        hash: $hash:literal, // Transaction hash
        data_path: $data_path:literal, // Path to the transaction blob
        version: $version:literal, // Transaction version
        timelock: $timelock:expr, // Transaction's timelock (use the real type `Timelock`)
        input_len: $input_len:literal, // Amount of inputs
        output_len: $output_len:literal, // Amount of outputs
        signatures_len: $signatures_len:literal, // Amount of signatures
    ) => {
        #[doc = concat!("Transaction with hash `", $hash, "`.")]
        ///
        /// ```rust
        #[doc = "# use cuprate_test_utils::data::*;"]
        #[doc = "use monero_serai::transaction::{Transaction, Timelock};"]
        #[doc = ""]
        #[doc = concat!("let tx = Transaction::read(&mut ", stringify!($name), ").unwrap();")]
        #[doc = ""]
        #[doc = concat!("assert_eq!(tx.prefix.version, ", $version, ");")]
        #[doc = concat!("assert_eq!(tx.prefix.timelock, ", stringify!($timelock), ");")]
        #[doc = concat!("assert_eq!(tx.prefix.inputs.len(), ", $input_len, ");")]
        #[doc = concat!("assert_eq!(tx.prefix.outputs.len(), ", $output_len, ");")]
        #[doc = concat!("assert_eq!(tx.signatures.len(), ", $signatures_len, ");")]
        #[doc = concat!("assert_eq!(hex::encode(tx.hash()), \"", $hash, "\")")]
        /// ```
        pub const $name: &[u8] = include_bytes!($data_path);
    };
}

const_tx_blob! {
    name: TX_3BC7FF,
    hash: "3bc7ff015b227e7313cc2e8668bfbb3f3acbee274a9c201d6211cf681b5f6bb1",
    data_path: "tx/3bc7ff015b227e7313cc2e8668bfbb3f3acbee274a9c201d6211cf681b5f6bb1.bin",
    version: 1,
    timelock: Timelock::Block(100_081),
    input_len: 1,
    output_len: 5,
    signatures_len: 0,
}

const_tx_blob! {
    name: TX_2180A8,
    hash: "2180a87f724702d37af087e22476297e818a73579ef7b7da947da963245202a3",
    data_path: "tx/2180a87f724702d37af087e22476297e818a73579ef7b7da947da963245202a3.bin",
    version: 1,
    timelock: Timelock::None,
    input_len: 19,
    output_len: 61,
    signatures_len: 19,
}

const_tx_blob! {
    name: TX_D7FEBD,
    hash: "d7febd16293799d9c6a8e0fe9199b8a0a3e0da5a8a165098937b60f0bbd582df",
    data_path: "tx/d7febd16293799d9c6a8e0fe9199b8a0a3e0da5a8a165098937b60f0bbd582df.bin",
    version: 1,
    timelock: Timelock::None,
    input_len: 46,
    output_len: 46,
    signatures_len: 46,
}

const_tx_blob! {
    name: TX_E2D393,
    hash: "e2d39395dd1625b2d707b98af789e7eab9d24c2bd2978ec38ef910961a8cdcee",
    data_path: "tx/e2d39395dd1625b2d707b98af789e7eab9d24c2bd2978ec38ef910961a8cdcee.bin",
    version: 2,
    timelock: Timelock::None,
    input_len: 1,
    output_len: 2,
    signatures_len: 0,
}

const_tx_blob! {
    name: TX_E57440,
    hash: "e57440ec66d2f3b2a5fa2081af40128868973e7c021bb3877290db3066317474",
    data_path: "tx/e57440ec66d2f3b2a5fa2081af40128868973e7c021bb3877290db3066317474.bin",
    version: 2,
    timelock: Timelock::None,
    input_len: 1,
    output_len: 2,
    signatures_len: 0,
}

const_tx_blob! {
    name: TX_B6B439,
    hash: "b6b4394d4ec5f08ad63267c07962550064caa8d225dd9ad6d739ebf60291c169",
    data_path: "tx/b6b4394d4ec5f08ad63267c07962550064caa8d225dd9ad6d739ebf60291c169.bin",
    version: 2,
    timelock: Timelock::None,
    input_len: 2,
    output_len: 2,
    signatures_len: 0,
}

const_tx_blob! {
    name: TX_9E3F73,
    hash: "9e3f73e66d7c7293af59c59c1ff5d6aae047289f49e5884c66caaf4aea49fb34",
    data_path: "tx/9e3f73e66d7c7293af59c59c1ff5d6aae047289f49e5884c66caaf4aea49fb34.bin",
    version: 1,
    timelock: Timelock::None,
    input_len: 2,
    output_len: 5,
    signatures_len: 2,
}

const_tx_blob! {
    name: TX_84D48D,
    hash: "84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66",
    data_path: "tx/84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66.bin",
    version: 2,
    timelock: Timelock::None,
    input_len: 2,
    output_len: 2,
    signatures_len: 0,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
