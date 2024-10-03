//! Signals for Cuprate state used throughout the binary.

use tokio::sync::RwLock;

/// Reorg lock.
///
/// A [`RwLock`] where a write lock is taken during a reorg and a read lock can be taken
/// for any operation which must complete without a reorg happening.
pub static REORG_LOCK: RwLock<()> = RwLock::const_new(());
