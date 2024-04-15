//! Constants holding raw Monero data.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Block
/// Block with height `202612` and hash `bbd604d2ba11ba27935e006ed39c9bfdd99b76bf4a50654bc1e1e61217962698`.
///
/// Serialized version: [`block_v1_tx513`](crate::data::free::block_v1_tx513).
///
/// ```rust
/// use monero_serai::{block::Block, transaction::Input};
///
/// let block = Block::read(&mut
///     cuprate_test_utils::data::BLOCK_BBD604
/// ).unwrap();
///
/// assert_eq!(block.header.major_version, 1);
/// assert_eq!(block.header.minor_version, 0);
/// assert_eq!(block.header.timestamp, 1409804570);
/// assert_eq!(block.header.nonce, 1073744198);
/// assert!(matches!(block.miner_tx.prefix.inputs[0], Input::Gen(202612)));
/// assert_eq!(block.txs.len(), 513);
///
/// assert_eq!(
///     hex::encode(block.hash()),
///     "bbd604d2ba11ba27935e006ed39c9bfdd99b76bf4a50654bc1e1e61217962698",
/// );
/// ```
pub const BLOCK_BBD604: &[u8] =
    include_bytes!("block/bbd604d2ba11ba27935e006ed39c9bfdd99b76bf4a50654bc1e1e61217962698.bin");

/// Block with height `2751506` and hash `f910435a5477ca27be1986c080d5476aeab52d0c07cf3d9c72513213350d25d4`.
///
/// Serialized version: [`block_v9_tx3`](crate::data::free::block_v9_tx3).
///
/// ```rust
/// use monero_serai::{block::Block, transaction::Input};
///
/// let block = Block::read(&mut
///     cuprate_test_utils::data::BLOCK_F91043
/// ).unwrap();
///
/// assert_eq!(block.header.major_version, 9);
/// assert_eq!(block.header.minor_version, 9);
/// assert_eq!(block.header.timestamp, 1545423190);
/// assert_eq!(block.header.nonce, 4123173351);
/// assert!(matches!(block.miner_tx.prefix.inputs[0], Input::Gen(1731606)));
/// assert_eq!(block.txs.len(), 3);
///
/// assert_eq!(
///     hex::encode(block.hash()),
///     "f910435a5477ca27be1986c080d5476aeab52d0c07cf3d9c72513213350d25d4",
/// );
/// ```
pub const BLOCK_F91043: &[u8] =
    include_bytes!("block/f910435a5477ca27be1986c080d5476aeab52d0c07cf3d9c72513213350d25d4.bin");

/// Block with height `2751506` and hash `43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428`.
///
/// Serialized version: [`block_v16_tx0`](crate::data::free::block_v16_tx0).
///
/// ```rust
/// use monero_serai::{block::Block, transaction::Input};
///
/// let block = Block::read(&mut
///     cuprate_test_utils::data::BLOCK_43BD1F
/// ).unwrap();
///
/// assert_eq!(block.header.major_version, 16);
/// assert_eq!(block.header.minor_version, 16);
/// assert_eq!(block.header.timestamp, 1667941829);
/// assert_eq!(block.header.nonce, 4110909056);
/// assert!(matches!(block.miner_tx.prefix.inputs[0], Input::Gen(2751506)));
/// assert_eq!(block.txs.len(), 0);
///
/// assert_eq!(
///     hex::encode(block.hash()),
///     "43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428",
/// );
/// ```
pub const BLOCK_43BD1F: &[u8] =
    include_bytes!("block/43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428.bin");

//---------------------------------------------------------------------------------------------------- Transaction
/// Transaction with hash `3bc7ff015b227e7313cc2e8668bfbb3f3acbee274a9c201d6211cf681b5f6bb1`.
///
/// Serialized version: [`tx_v1_sig0`](crate::data::free::tx_v1_sig0).
///
/// ```rust
/// use monero_serai::transaction::{Transaction, Timelock};
///
/// let tx = Transaction::read(&mut
///     cuprate_test_utils::data::TX_3BC7FF
/// ).unwrap();
///
/// assert_eq!(tx.prefix.version, 1);
/// assert_eq!(tx.prefix.timelock, Timelock::Block(100_081));
/// assert_eq!(tx.prefix.inputs.len(), 1);
/// assert_eq!(tx.prefix.outputs.len(), 5);
/// assert_eq!(tx.signatures.len(), 0);
///
/// assert_eq!(
///     hex::encode(tx.hash()),
///     "3bc7ff015b227e7313cc2e8668bfbb3f3acbee274a9c201d6211cf681b5f6bb1",
/// );
/// ```
pub const TX_3BC7FF: &[u8] =
    include_bytes!("tx/3bc7ff015b227e7313cc2e8668bfbb3f3acbee274a9c201d6211cf681b5f6bb1.bin");

/// Transaction with hash `9e3f73e66d7c7293af59c59c1ff5d6aae047289f49e5884c66caaf4aea49fb34`.
///
/// Serialized version: [`tx_v1_sig2`](crate::data::free::tx_v1_sig2).
///
/// ```rust
/// use monero_serai::transaction::{Transaction, Timelock};
///
/// let tx = Transaction::read(&mut
///     cuprate_test_utils::data::TX_9E3F73
/// ).unwrap();
///
/// assert_eq!(tx.prefix.version, 1);
/// assert_eq!(tx.prefix.timelock, Timelock::None);
/// assert_eq!(tx.prefix.inputs.len(), 2);
/// assert_eq!(tx.prefix.outputs.len(), 5);
/// assert_eq!(tx.signatures.len(), 2);
///
/// assert_eq!(
///     hex::encode(tx.hash()),
///     "9e3f73e66d7c7293af59c59c1ff5d6aae047289f49e5884c66caaf4aea49fb34",
/// );
/// ```
pub const TX_9E3F73: &[u8] =
    include_bytes!("tx/9e3f73e66d7c7293af59c59c1ff5d6aae047289f49e5884c66caaf4aea49fb34.bin");

/// Transaction with hash `84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66`.
///
/// Serialized version: [`tx_v2_rct3`](crate::data::free::tx_v2_rct3).
///
/// ```rust
/// use monero_serai::transaction::{Transaction, Timelock};
///
/// let tx = Transaction::read(&mut
///     cuprate_test_utils::data::TX_84D48D
/// ).unwrap();
///
/// assert_eq!(tx.prefix.version, 2);
/// assert_eq!(tx.prefix.timelock, Timelock::None);
/// assert_eq!(tx.prefix.inputs.len(), 2);
/// assert_eq!(tx.prefix.outputs.len(), 2);
/// assert_eq!(tx.signatures.len(), 0);
///
/// assert_eq!(
///     hex::encode(tx.hash()),
///     "84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66",
/// );
/// ```
pub const TX_84D48D: &[u8] =
    include_bytes!("tx/84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66.bin");

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
