//! Output functions.

use std::collections::HashMap;
use std::io;
use std::ops::AddAssign;
use fjall::Readable;
//---------------------------------------------------------------------------------------------------- Import
use cuprate_helper::{cast::u32_to_usize, crypto::compute_zero_commitment, map::u64_to_timelock};
use cuprate_types::OutputOnChain;
use monero_oxide::{io::CompressedPoint, transaction::Timelock};

use crate::database::RCT_OUTPUTS;
use crate::error::{BlockchainError, DbResult};
use crate::types::TxId;
use crate::Blockchain;
use crate::{
    ops::{
        macros::{doc_add_block_inner_invariant, doc_error},
        tx::get_tx_from_id,
    },
    types::{Amount, AmountIndex, Output, PreRctOutputId, RctOutput},
};

fn pre_rct_output_id_to_bytes(pre_rct_output_id: &PreRctOutputId) -> [u8; 16] {
    let mut buf = [0; 16];
    buf[..8].copy_from_slice(&pre_rct_output_id.amount.to_be_bytes());
    buf[8..].copy_from_slice(&pre_rct_output_id.amount_index.to_be_bytes());
    buf
}

fn bytes_to_pre_rct_output_id(bytes: &[u8; 16]) -> PreRctOutputId {
    PreRctOutputId {
        amount: Amount::from_be_bytes(bytes[..8].try_into().unwrap()),
        amount_index: AmountIndex::from_be_bytes(bytes[8..].try_into().unwrap()),
    }
}

//---------------------------------------------------------------------------------------------------- Pre-RCT Outputs
/// Add a Pre-RCT [`Output`] to the database.
///
/// Upon [`Ok`], this function returns the [`PreRctOutputId`] that
/// can be used to lookup the `Output` in [`get_output()`].
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
#[inline]
pub fn add_output(
    db: &Blockchain,
    amount: Amount,
    output: &Output,
    tx_rw: &mut fjall::SingleWriterWriteTx,
    pre_rct_numb_outputs_cache: &mut HashMap<Amount, u64>,
) -> DbResult<PreRctOutputId> {
    let num_outputs = pre_rct_numb_outputs_cache.entry(amount).or_insert_with(|| {
        0 //TODO
    });

    let pre_rct_output_id = PreRctOutputId {
        amount,
        // The new `amount_index` is the length of amount of outputs with same amount.
        amount_index:* num_outputs,
    };

    tx_rw.insert(&db.pre_rct_outputs_fjall, pre_rct_output_id_to_bytes(&pre_rct_output_id), bytemuck::bytes_of(output));

    num_outputs.add_assign(1);

    Ok(pre_rct_output_id)
}

/// Remove a Pre-RCT [`Output`] from the database.
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
#[inline]
pub fn remove_output(db: &Blockchain, amount: Amount, tx_rw: &mut fjall::SingleWriterWriteTx) -> DbResult<()> {
    todo!()
}

/// Retrieve a Pre-RCT [`Output`] from the database.
#[doc = doc_error!()]
#[inline]
pub fn get_output(db: &Blockchain, pre_rct_output_id: &PreRctOutputId, tx_ro: &fjall::Snapshot) -> DbResult<Output> {
    let output  = tx_ro.get(&db.pre_rct_outputs_fjall, pre_rct_output_id_to_bytes(pre_rct_output_id)).expect("TODO").ok_or(BlockchainError::NotFound)?;

    Ok(bytemuck::pod_read_unaligned(output.as_ref()))
}

/// How many pre-RCT [`Output`]s are there?
///
/// This returns the amount of pre-RCT outputs currently stored.
#[doc = doc_error!()]
#[inline]
pub fn get_num_outputs(db: &Blockchain, tx_ro: &fjall::Snapshot) -> DbResult<u64> {
    Ok(tx_ro.len(&db.pre_rct_outputs_fjall).expect("TODO") as u64)
}

#[inline]
pub fn get_num_outputs_with_amount(db: &Blockchain, tx_ro: &fjall::Snapshot, amount: Amount) -> DbResult<u64> {
    let last_out = tx_ro.prefix(&db.pre_rct_outputs_fjall, &amount.to_be_bytes()).next_back();

    last_out.map_or(Ok(0), |o| Ok(u64::from_be_bytes(o.key().expect("TODO")[8..].try_into().unwrap()) + 1))
}

//-xe--------------------------------------------------------- Mapping functions
/// Map an [`Output`] to a [`cuprate_types::OutputOnChain`].
#[doc = doc_error!()]
pub fn output_to_output_on_chain(
    output: &Output,
    amount: Amount,
    get_txid: bool,
    tapes: &tapes::TapesReadTransaction,
    db: &Blockchain,
) -> DbResult<OutputOnChain> {
    let commitment = compute_zero_commitment(amount);

    let key = CompressedPoint(output.key);

    let txid = if get_txid {
        let txid = get_tx_from_id(&output.tx_idx, tapes, db)?.hash();

        Some(txid)
    } else {
        None
    };

    Ok(OutputOnChain {
        height: output.height,
        time_lock: u64_to_timelock(output.timelock),
        key,
        commitment,
        txid,
    })
}

/// Map an [`RctOutput`] to a [`cuprate_types::OutputOnChain`].
///
/// # Panics
/// This function will panic if `rct_output`'s `commitment` fails to decompress
/// into a valid Ed25519 point.
///
/// This should normally not happen as commitments that
/// are stored in the database should always be valid.
#[doc = doc_error!()]
#[inline]
pub fn rct_output_to_output_on_chain(
    rct_output: &RctOutput,
    get_txid: bool,
    tapes: &tapes::TapesReadTransaction,
    db: &Blockchain,
) -> DbResult<OutputOnChain> {
    // INVARIANT: Commitments stored are valid when stored by the database.
    let commitment = CompressedPoint(rct_output.commitment);

    let key = CompressedPoint(rct_output.key);

    let txid = if get_txid {
        let txid = get_tx_from_id(&rct_output.tx_idx, tapes, db)?.hash();

        Some(txid)
    } else {
        None
    };

    Ok(OutputOnChain {
        height: rct_output.height,
        time_lock: u64_to_timelock(rct_output.timelock),
        key,
        commitment,
        txid,
    })
}

/// Map an [`PreRctOutputId`] to an [`OutputOnChain`].
///
/// Note that this still support RCT outputs, in that case, [`PreRctOutputId::amount`] should be `0`.
#[doc = doc_error!()]
pub fn id_to_output_on_chain(
    db: &Blockchain,
    id: &PreRctOutputId,
    get_txid: bool,
    tx_ro: &fjall::Snapshot,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<OutputOnChain> {
    // v2 transactions.
    if id.amount == 0 {
        let rct_output = tapes.read_entry(&db.rct_outputs, id.amount_index)?.ok_or(BlockchainError::NotFound)?;
        let output_on_chain = rct_output_to_output_on_chain(&rct_output, get_txid, tapes, db)?;

        Ok(output_on_chain)
    } else {
        // v1 transactions.
        let output = get_output(db, id, tx_ro)?;
        let output_on_chain = output_to_output_on_chain(&output, id.amount, get_txid, tapes, db)?;

        Ok(output_on_chain)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;

    use cuprate_database::{Env, EnvInner};

    use crate::{
        tables::{OpenTables, Tables, TablesMut},
        tests::{assert_all_tables_are_empty, tmp_concrete_env, AssertTableLen},
        types::OutputFlags,
    };

    /// Dummy `Output`.
    const OUTPUT: Output = Output {
        key: [44; 32],
        height: 0,
        output_flags: OutputFlags::NON_ZERO_UNLOCK_TIME,
        tx_idx: 0,
    };

    /// Dummy `RctOutput`.
    const RCT_OUTPUT: RctOutput = RctOutput {
        key: [88; 32],
        height: 1,
        output_flags: OutputFlags::empty(),
        tx_idx: 1,
        commitment: [100; 32],
    };

    /// Dummy `Amount`
    const AMOUNT: Amount = 22;

    /// Tests all above output functions when only inputting `Output` data (no Block).
    ///
    /// Note that this doesn't test the correctness of values added, as the
    /// functions have a pre-condition that the caller handles this.
    ///
    /// It simply tests if the proper tables are mutated, and if the data
    /// stored and retrieved is the same.
    #[test]
    fn all_output_functions() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let tx_rw = env_inner.tx_rw().unwrap();
        let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

        // Assert length is correct.
        assert_eq!(get_num_outputs(tables.outputs()).unwrap(), 0);
        assert_eq!(get_rct_num_outputs(tables.rct_outputs()).unwrap(), 0);

        // Add outputs.
        let pre_rct_output_id = add_output(AMOUNT, &OUTPUT, &mut tables).unwrap();
        let amount_index = add_rct_output(&RCT_OUTPUT, tables.rct_outputs_mut()).unwrap();

        assert_eq!(
            pre_rct_output_id,
            PreRctOutputId {
                amount: AMOUNT,
                amount_index: 0,
            }
        );

        // Assert all reads of the outputs are OK.
        {
            // Assert proper tables were added to.
            AssertTableLen {
                block_infos: 0,
                block_header_blobs: 0,
                block_txs_hashes: 0,
                block_heights: 0,
                key_images: 0,
                num_outputs: 1,
                pruned_tx_blobs: 0,
                prunable_hashes: 0,
                outputs: 1,
                prunable_tx_blobs: 0,
                rct_outputs: 1,
                tx_blobs: 0,
                tx_ids: 0,
                tx_heights: 0,
                tx_unlock_time: 0,
            }
            .assert(&tables);

            // Assert length is correct.
            assert_eq!(get_num_outputs(tables.outputs()).unwrap(), 1);
            assert_eq!(get_rct_num_outputs(tables.rct_outputs()).unwrap(), 1);
            assert_eq!(1, tables.num_outputs().get(&AMOUNT).unwrap());

            // Assert value is save after retrieval.
            assert_eq!(
                OUTPUT,
                get_output(&pre_rct_output_id, tables.outputs()).unwrap(),
            );

            assert_eq!(
                RCT_OUTPUT,
                get_rct_output(&amount_index, tables.rct_outputs()).unwrap(),
            );
        }

        // Remove the outputs.
        {
            remove_output(&pre_rct_output_id, &mut tables).unwrap();
            remove_rct_output(&amount_index, tables.rct_outputs_mut()).unwrap();

            // Assert value no longer exists.
            assert!(matches!(
                get_output(&pre_rct_output_id, tables.outputs()),
                Err(RuntimeError::KeyNotFound)
            ));
            assert!(matches!(
                get_rct_output(&amount_index, tables.rct_outputs()),
                Err(RuntimeError::KeyNotFound)
            ));

            // Assert length is correct.
            assert_eq!(get_num_outputs(tables.outputs()).unwrap(), 0);
            assert_eq!(get_rct_num_outputs(tables.rct_outputs()).unwrap(), 0);
        }

        assert_all_tables_are_empty(&env);
    }
}
