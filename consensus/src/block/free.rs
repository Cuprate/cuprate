//! Free functions for block verification
use std::collections::HashMap;

use monero_serai::block::Block;

use cuprate_types::TransactionVerificationData;

use crate::ExtendedConsensusError;

/// Orders the [`TransactionVerificationData`] list the same as it appears in [`Block::transactions`]
pub(crate) fn order_transactions(
    block: &Block,
    txs: &mut [TransactionVerificationData],
) -> Result<(), ExtendedConsensusError> {
    if block.transactions.len() != txs.len() {
        return Err(ExtendedConsensusError::TxsIncludedWithBlockIncorrect);
    }

    for (i, tx_hash) in block.transactions.iter().enumerate() {
        if tx_hash != &txs[i].tx_hash {
            let at_index = txs[i..]
                .iter()
                .position(|tx| &tx.tx_hash == tx_hash)
                .ok_or(ExtendedConsensusError::TxsIncludedWithBlockIncorrect)?;

            // The above `position` will give an index from inside its view of the slice so we need to add the difference.
            txs.swap(i, i + at_index);
        }
    }

    debug_assert!(
        block
            .transactions
            .iter()
            .zip(txs.iter())
            .all(|(tx_hash, tx)| tx_hash == &tx.tx_hash)
    );

    Ok(())
}

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
