//! Free functions to access data.

//---------------------------------------------------------------------------------------------------- Import
use std::sync::OnceLock;

use monero_serai::transaction::Transaction;

use crate::data::TX_84D48DC11EC91950F8B70A85AF9DB91FE0C8ABEF71EF5DB08304F7344B99EA66;

//---------------------------------------------------------------------------------------------------- Constants
/// `OnceLock` holding the below fn's data.
static TX: OnceLock<Transaction> = OnceLock::new();
/// Return a real `Transaction` struct.
///
/// Uses data from [`TX_84D48DC11EC91950F8B70A85AF9DB91FE0C8ABEF71EF5DB08304F7344B99EA66`].
#[allow(clippy::missing_panics_doc)] // this shouldn't panic
pub fn tx() -> Transaction {
    TX.get_or_init(|| {
        #[allow(const_item_mutation)] // `R: Read` needs `&mut self`
        Transaction::read(&mut TX_84D48DC11EC91950F8B70A85AF9DB91FE0C8ABEF71EF5DB08304F7344B99EA66)
            .unwrap()
    })
    .clone()
}
