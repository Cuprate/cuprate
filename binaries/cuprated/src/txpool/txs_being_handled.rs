use std::sync::Arc;

use dashmap::DashSet;

/// A set of txs currently being handled, shared between instances of the incoming tx handler.
#[derive(Clone)]
pub struct TxsBeingHandled(Arc<DashSet<[u8; 32]>>);

impl TxsBeingHandled {
    /// Create a new [`TxsBeingHandled`]
    pub fn new() -> Self {
        Self(Arc::new(DashSet::new()))
    }

    /// Create a new [`TxsBeingHandledLocally`] that will keep track of txs being handled in a request.
    pub fn local_tracker(&self) -> TxsBeingHandledLocally {
        TxsBeingHandledLocally {
            txs_being_handled: self.clone(),
            txs: vec![],
        }
    }
}

/// A tracker of txs being handled in a single request. This will add the txs to the global [`TxsBeingHandled`]
/// tracker as well.
///
/// When this is dropped the txs will be removed from [`TxsBeingHandled`].
pub struct TxsBeingHandledLocally {
    txs_being_handled: TxsBeingHandled,
    txs: Vec<[u8; 32]>,
}

impl TxsBeingHandledLocally {
    /// Try add a tx to the map from its [`transaction_blob_hash`](cuprate_txpool::transaction_blob_hash).
    ///
    /// Returns `true` if the tx was added and `false` if another task is already handling this tx.
    pub fn try_add_tx(&mut self, tx_blob_hash: [u8; 32]) -> bool {
        if !self.txs_being_handled.0.insert(tx_blob_hash) {
            return false;
        }

        self.txs.push(tx_blob_hash);
        true
    }
}

impl Drop for TxsBeingHandledLocally {
    fn drop(&mut self) {
        for hash in &self.txs {
            self.txs_being_handled.0.remove(hash);
        }
    }
}
