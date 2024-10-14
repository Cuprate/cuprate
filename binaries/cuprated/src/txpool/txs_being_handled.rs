use dashmap::DashSet;
use sha3::{Digest, Sha3_256};
use std::sync::Arc;

pub fn tx_blob_hash(tx_bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(tx_bytes);
    hasher.finalize().into()
}

#[derive(Clone)]
pub struct TxsBeingHandled(Arc<DashSet<[u8; 32]>>);

impl TxsBeingHandled {
    pub fn new() -> Self {
        Self(Arc::new(DashSet::new()))
    }

    pub fn local_tracker(&self) -> TxBeingHandledLocally {
        TxBeingHandledLocally {
            txs_being_handled: self.clone(),
            txs: vec![],
        }
    }
}

pub struct TxBeingHandledLocally {
    txs_being_handled: TxsBeingHandled,
    txs: Vec<[u8; 32]>,
}

impl TxBeingHandledLocally {
    pub fn try_add_tx(&mut self, tx_blob_hash: [u8; 32]) -> bool {
        if !self.txs_being_handled.0.insert(tx_blob_hash) {
            return false;
        }

        self.txs.push(tx_blob_hash);
        true
    }
}

impl Drop for TxBeingHandledLocally {
    fn drop(&mut self) {
        for hash in &self.txs {
            self.txs_being_handled.0.remove(hash);
        }
    }
}
