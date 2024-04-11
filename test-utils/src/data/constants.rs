//! Constants holding raw Monero data.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Block
/// Block with height `202612` and hash `bbd604d2ba11ba27935e006ed39c9bfdd99b76bf4a50654bc1e1e61217962698`.
///
/// ```rust
/// use monero_serai::{block::Block, transaction::Input};
///
/// let block = Block::read(&mut
///     cuprate_test_utils::data::BLOCK_202612
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
pub const BLOCK_202612: &[u8] = include_bytes!("block/202612.bin");

//---------------------------------------------------------------------------------------------------- Transaction
/// Transaction with hash `84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66`.
///
/// ```rust
/// use monero_serai::transaction::{Transaction, Timelock};
///
/// let tx = Transaction::read(&mut
///     cuprate_test_utils::data::TX_84D48DC11EC91950F8B70A85AF9DB91FE0C8ABEF71EF5DB08304F7344B99EA66
/// ).unwrap();
///
/// assert_eq!(tx.prefix.version, 2);
/// assert_eq!(tx.prefix.timelock, Timelock::None);
/// assert_eq!(tx.prefix.inputs.len(), 2);
/// assert_eq!(tx.prefix.outputs.len(), 2);
///
/// assert_eq!(
///     hex::encode(tx.hash()),
///     "84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66",
/// );
/// ```
pub const TX_84D48DC11EC91950F8B70A85AF9DB91FE0C8ABEF71EF5DB08304F7344B99EA66: &[u8] =
    include_bytes!("tx/84d48dc11ec91950f8b70a85af9db91fe0c8abef71ef5db08304f7344b99ea66.bin");

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
