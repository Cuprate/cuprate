//! Free functions for block verification
use std::collections::HashMap;

use monero_serai::block::Block;

use cuprate_types::TransactionVerificationData;

use crate::ExtendedConsensusError;

/// Returns a list of transactions, pulled from `txs` in the order they are in the [`Block`].
///
/// Will error if a tx need is not in `txs` or if `txs` contain more txs than needed.
pub(crate) fn pull_ordered_transactions(
    block: &Block,
    mut txs: HashMap<[u8; 32], TransactionVerificationData>,
) -> Result<Vec<TransactionVerificationData>, ExtendedConsensusError> {
    if block.transactions.len() != txs.len() {
        return Err(ExtendedConsensusError::TxsIncludedWithBlockIncorrect);
    }

    let mut ordered_txs = Vec::with_capacity(txs.len());

    if !block.transactions.is_empty() {
        for tx_hash in &block.transactions {
            let tx = txs
                .remove(tx_hash)
                .ok_or(ExtendedConsensusError::TxsIncludedWithBlockIncorrect)?;
            ordered_txs.push(tx);
        }
        drop(txs);
    }

    Ok(ordered_txs)
}
