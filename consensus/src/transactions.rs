use monero_serai::transaction::Transaction;
use sha3::{Digest, Keccak256};

use crate::{hardforks::HardFork, ConsensusError, Database};

mod inputs;
mod outputs;
mod signatures;
mod time_lock;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TxVersion {
    RingSignatures,
    RingCT,
}

impl TxVersion {
    pub fn from_raw(version: u64) -> Result<TxVersion, ConsensusError> {
        match version {
            1 => Ok(TxVersion::RingSignatures),
            2 => Ok(TxVersion::RingCT),
            _ => Err(ConsensusError::TransactionVersionInvalid),
        }
    }
}

/// Data needed to verify a transaction.
///
pub struct TransactionVerificationData {
    tx: Transaction,
    version: TxVersion,
    tx_blob: Vec<u8>,
    tx_weight: usize,
    tx_hash: [u8; 32],
    rings: signatures::Rings,
}

impl TransactionVerificationData {
    pub fn new(
        tx: Transaction,
        rings: signatures::Rings,
    ) -> Result<TransactionVerificationData, ConsensusError> {
        let tx_blob = tx.serialize();

        Ok(TransactionVerificationData {
            tx_hash: Keccak256::digest(&tx_blob).into(),
            tx_blob,
            tx_weight: tx.weight(),
            rings,
            version: TxVersion::from_raw(tx.prefix.version)?,
            tx,
        })
    }

    pub async fn batch_new<D: Database + Clone>(
        txs: Vec<Transaction>,
        hf: &HardFork,
        database: D,
    ) -> Result<Vec<TransactionVerificationData>, ConsensusError> {
        let rings = signatures::batch_get_rings(&txs, hf, database.clone()).await?;

        txs.into_iter()
            .zip(rings.into_iter())
            .map(|(tx, ring)| TransactionVerificationData::new(tx, ring))
            .collect()
    }
}
