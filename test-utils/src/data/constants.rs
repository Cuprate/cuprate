//! General constants.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Transaction
/// Block with height `202612`.
///
/// FIXME: doc test asserting fields.
pub const BLOCK_202612: &[u8] = include_bytes!("block/202612.bin");

//---------------------------------------------------------------------------------------------------- Transaction
/// Transaction with hash `84D48DC11EC91950F8B70A85AF9DB91FE0C8ABEF71EF5DB08304F7344B99EA66`.
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
/// ```
pub const TX_84D48DC11EC91950F8B70A85AF9DB91FE0C8ABEF71EF5DB08304F7344B99EA66: &[u8] =
    include_bytes!("tx/84D48DC11EC91950F8B70A85AF9DB91FE0C8ABEF71EF5DB08304F7344B99EA66.bin");

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}
