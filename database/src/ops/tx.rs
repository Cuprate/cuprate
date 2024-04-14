//! Transactions.

//---------------------------------------------------------------------------------------------------- Import
use cuprate_types::{OutputOnChain, TransactionVerificationData, VerifiedBlockInformation};
use monero_pruning::PruningSeed;
use monero_serai::transaction::{Timelock, Transaction};

use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::{
        macros::{doc_add_block_inner_invariant, doc_error},
        property::get_blockchain_pruning_seed,
    },
    tables::{
        BlockBlobs, BlockHeights, BlockInfos, KeyImages, NumOutputs, Outputs, PrunableHashes,
        PrunableTxBlobs, PrunedTxBlobs, RctOutputs, Tables, TablesMut, TxBlobs, TxHeights, TxIds,
        TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        BlockHash, BlockHeight, BlockInfo, KeyImage, Output, PreRctOutputId, RctOutput, TxBlob,
        TxHash, TxId,
    },
    StorableVec,
};

//---------------------------------------------------------------------------------------------------- Private
/// TODO
///
/// TODO: document this add to the latest block height.
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
#[inline]
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn add_tx(tx: &Transaction, tables: &mut impl TablesMut) -> Result<TxId, RuntimeError> {
    let tx_id = get_num_tx(tables.tx_ids_mut())?;
    let height = crate::ops::blockchain::chain_height(tables.block_heights_mut())?;

    tables.tx_ids_mut().put(&tx.hash(), &tx_id)?;
    tables.tx_heights_mut().put(&tx_id, &height)?;
    tables
        .tx_blobs_mut()
        .put(&tx_id, &StorableVec(tx.serialize()))?;

    // Timelocks.
    //
    // Height/time is not differentiated via type, but rather:
    // "height is any value less than 500_000_000 and timestamp is any value above"
    // so the `u64/usize` is stored without any tag.
    //
    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1558504285>
    match tx.prefix.timelock {
        Timelock::None => (),
        Timelock::Block(height) => tables.tx_unlock_time_mut().put(&tx_id, &(height as u64))?,
        Timelock::Time(time) => tables.tx_unlock_time_mut().put(&tx_id, &time)?,
    }

    // SOMEDAY: implement pruning after `monero-serai` does.
    // if let PruningSeed::Pruned(decompressed_pruning_seed) = get_blockchain_pruning_seed()? {
    // SOMEDAY: what to store here? which table?
    // }

    Ok(tx_id)
}

/// TODO
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
#[inline]
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn remove_tx(
    tx_hash: &TxHash,
    tables: &mut impl TablesMut,
) -> Result<(TxId, TxBlob), RuntimeError> {
    let tx_id = tables.tx_ids_mut().take(tx_hash)?;
    tables.tx_heights_mut().delete(&tx_id)?;

    // SOMEDAY: implement pruning after `monero-serai` does.
    // table_prunable_hashes.delete(&tx_id)?;
    // table_prunable_tx_blobs.delete(&tx_id)?;
    // if let PruningSeed::Pruned(decompressed_pruning_seed) = get_blockchain_pruning_seed()? {
    // SOMEDAY: what to remove here? which table?
    // }

    match tables.tx_unlock_time_mut().delete(&tx_id) {
        Err(RuntimeError::KeyNotFound) | Ok(()) => (),
        // An actual error occurred, return.
        Err(e) => return Err(e),
    };

    let tx_blob = tables.tx_blobs_mut().take(&tx_id)?;

    Ok((tx_id, tx_blob))
}

//---------------------------------------------------------------------------------------------------- `get_tx_*`
/// TODO
///
#[doc = doc_error!()]
#[inline]
pub fn get_tx(
    tx_hash: &TxHash,
    table_tx_ids: &impl DatabaseRo<TxIds>,
    table_tx_blobs: &impl DatabaseRo<TxBlobs>,
) -> Result<Transaction, RuntimeError> {
    get_tx_from_id(&table_tx_ids.get(tx_hash)?, table_tx_blobs)
}

/// TODO
///
#[doc = doc_error!()]
#[inline]
pub fn get_tx_from_id(
    tx_id: &TxId,
    table_tx_blobs: &impl DatabaseRo<TxBlobs>,
) -> Result<Transaction, RuntimeError> {
    let tx_blob = table_tx_blobs.get(tx_id)?.0;
    Ok(Transaction::read(&mut tx_blob.as_slice())?)
}

//----------------------------------------------------------------------------------------------------
/// TODO
///
#[doc = doc_error!()]
#[inline]
pub fn get_num_tx(table_tx_ids: &impl DatabaseRo<TxIds>) -> Result<u64, RuntimeError> {
    table_tx_ids.len()
}

//----------------------------------------------------------------------------------------------------
/// TODO
///
#[doc = doc_error!()]
#[inline]
pub fn tx_exists(
    tx_hash: &TxHash,
    table_tx_ids: &impl DatabaseRo<TxIds>,
) -> Result<bool, RuntimeError> {
    table_tx_ids.contains(tx_hash)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
#[allow(clippy::significant_drop_tightening)]
mod test {
    use super::*;
    use crate::{
        tests::{assert_all_tables_are_empty, tmp_concrete_env},
        Env,
    };
    use cuprate_test_utils::data::{tx_v1_sig0, tx_v1_sig2, tx_v2_rct3};

    /// Tests all above tx functions.
    #[test]
    fn all_tx_functions() {
        let (env, tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        // Monero `Transaction`, not database tx.
        let txs = [tx_v1_sig0(), tx_v1_sig2(), tx_v2_rct3()];

        // Add transactions.
        let tx_ids = {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            let tx_ids = txs
                .iter()
                .map(|tx| add_tx(tx, &mut tables).unwrap())
                .collect::<Vec<TxId>>();

            drop(tables);
            TxRw::commit(tx_rw).unwrap();

            tx_ids
        };

        // Assert all reads of the transactions are OK.
        let tx_hashes = {
            let tx_ro = env_inner.tx_ro().unwrap();
            let tables = env_inner.open_tables(&tx_ro).unwrap();

            // Assert only the proper tables were added to.
            assert_eq!(tables.block_infos().len().unwrap(), 0);
            assert_eq!(tables.block_blobs().len().unwrap(), 0);
            assert_eq!(tables.block_heights().len().unwrap(), 0);
            assert_eq!(tables.key_images().len().unwrap(), 0);
            assert_eq!(tables.num_outputs().len().unwrap(), 0);
            assert_eq!(tables.pruned_tx_blobs().len().unwrap(), 0);
            assert_eq!(tables.prunable_hashes().len().unwrap(), 0);
            assert_eq!(tables.outputs().len().unwrap(), 0);
            assert_eq!(tables.prunable_tx_blobs().len().unwrap(), 0);
            assert_eq!(tables.rct_outputs().len().unwrap(), 0);
            assert_eq!(tables.tx_blobs().len().unwrap(), 3);
            assert_eq!(tables.tx_ids().len().unwrap(), 3);
            assert_eq!(tables.tx_heights().len().unwrap(), 3);
            assert_eq!(tables.tx_unlock_time().len().unwrap(), 1); // only 1 has a timelock

            // Both from ID and hash should result in getting the same transaction.
            let mut tx_hashes = vec![];
            for (i, tx_id) in tx_ids.iter().enumerate() {
                let tx_get_from_id = get_tx_from_id(tx_id, tables.tx_blobs()).unwrap();
                let tx_hash = tx_get_from_id.hash();
                let tx_get = get_tx(&tx_hash, tables.tx_ids(), tables.tx_blobs()).unwrap();

                assert_eq!(tx_get_from_id, tx_get);
                assert_eq!(&tx_get, txs[i]);
                assert!(tx_exists(&tx_hash, tables.tx_ids()).unwrap());

                tx_hashes.push(tx_hash);
            }

            tx_hashes
        };

        // Remove the transactions.
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            for tx_hash in tx_hashes {
                let (tx_id, _) = remove_tx(&tx_hash, &mut tables).unwrap();
                assert!(matches!(
                    get_tx_from_id(&tx_id, tables.tx_blobs()),
                    Err(RuntimeError::KeyNotFound)
                ));
            }

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        assert_all_tables_are_empty(&env);
    }
}
