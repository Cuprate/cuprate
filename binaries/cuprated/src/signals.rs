//! Signals for Cuprate state used throughout the binary.

use tokio::sync::RwLock;

/// Reorg lock.
///
/// A [`RwLock`] where a write lock is taken during a reorg and a read lock can be taken
/// for any operation which must complete without a reorg happening.
///
/// Currently, the only operation that needs to take a read lock is adding txs to the tx-pool,
/// this can potentially be removed in the future, see: TODO
pub static REORG_LOCK: RwLock<()> = RwLock::const_new(());
