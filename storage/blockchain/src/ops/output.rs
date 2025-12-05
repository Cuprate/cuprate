//! Output functions.

use heed::PutFlags;
//---------------------------------------------------------------------------------------------------- Import
use monero_oxide::{io::CompressedPoint, transaction::Timelock};
use tapes::MmapFile;
use cuprate_helper::{cast::u32_to_usize, crypto::compute_zero_commitment, map::u64_to_timelock};
use cuprate_types::OutputOnChain;

use crate::database::{PRE_RCT_OUTPUTS, RCT_OUTPUTS, TX_OUTPUTS};
use crate::error::{BlockchainError, DbResult};
use crate::types::TxId;
use crate::{
    ops::{
        macros::{doc_add_block_inner_invariant, doc_error},
        tx::get_tx_from_id,
    },
    types::{Amount, AmountIndex, Output, PreRctOutputId, RctOutput},
};

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
    amount: Amount,
    key: [u8; 32],
    height: usize,
    timelock: u64,
    tx_idx: TxId,
    tx_rw: &mut heed::RwTxn,
) -> DbResult<PreRctOutputId> {
    let num_outputs = if let Some(mut rw_iter) = PRE_RCT_OUTPUTS
        .get()
        .unwrap()
        .get_duplicates(tx_rw, &amount)?
    {
        rw_iter.last().unwrap()?.1.amount_index + 1
    } else {
        0
    };

    let pre_rct_output_id = PreRctOutputId {
        amount,
        // The new `amount_index` is the length of amount of outputs with same amount.
        amount_index: num_outputs,
    };

    PRE_RCT_OUTPUTS.get().unwrap().put_with_flags(
        tx_rw,
        PutFlags::APPEND_DUP,
        &amount,
        &Output {
            amount_index: num_outputs,
            key,
            height,
            timelock,
            tx_idx,
        },
    )?;

    Ok(pre_rct_output_id)
}

/// Remove a Pre-RCT [`Output`] from the database.
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
#[inline]
pub fn remove_output(amount: Amount, tx_rw: &mut heed::RwTxn) -> DbResult<()> {
    PRE_RCT_OUTPUTS.get().unwrap().delete_one_duplicate(
        tx_rw,
        &amount,
        &Output {
            amount_index: get_num_outputs_with_amount(tx_rw, amount)? - 1,
            key: [0; 32],
            height: 0,
            timelock: 0,
            tx_idx: 0,
        },
    )?;

    Ok(())
}

/// Retrieve a Pre-RCT [`Output`] from the database.
#[doc = doc_error!()]
#[inline]
pub fn get_output(pre_rct_output_id: &PreRctOutputId, tx_ro: &heed::RoTxn) -> DbResult<Output> {
    PRE_RCT_OUTPUTS
        .get()
        .unwrap()
        .get_duplicate(
            tx_ro,
            &pre_rct_output_id.amount,
            &Output {
                amount_index: pre_rct_output_id.amount_index,
                key: [0; 32],
                height: 0,
                timelock: 0,
                tx_idx: 0,
            },
        )?
        .ok_or(BlockchainError::NotFound)
}

/// How many pre-RCT [`Output`]s are there?
///
/// This returns the amount of pre-RCT outputs currently stored.
#[doc = doc_error!()]
#[inline]
pub fn get_num_outputs(tx_ro: &heed::RoTxn) -> DbResult<u64> {
    Ok(PRE_RCT_OUTPUTS.get().unwrap().len(tx_ro)?)
}

#[inline]
pub fn get_num_outputs_with_amount(tx_ro: &heed::RoTxn, amount: Amount) -> DbResult<u64> {
    let outs = PRE_RCT_OUTPUTS
        .get()
        .unwrap()
        .get_duplicates(tx_ro, &amount)?;

    outs.map_or(Ok(0), |o| Ok(o.last().unwrap()?.1.amount_index + 1))
}

//---------------------------------------------------------------------------------------------------- Mapping functions
/// Map an [`Output`] to a [`cuprate_types::OutputOnChain`].
#[doc = doc_error!()]
pub fn output_to_output_on_chain(
    output: &Output,
    amount: Amount,
    get_txid: bool,
    tapes: &tapes::Reader<MmapFile>,
) -> DbResult<OutputOnChain> {
    let commitment = compute_zero_commitment(amount);

    let key = CompressedPoint(output.key);

    let txid = if get_txid {
        let txid = get_tx_from_id(&output.tx_idx, tapes)?.hash();

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
    tapes: &tapes::Reader<MmapFile>,
) -> DbResult<OutputOnChain> {
    // INVARIANT: Commitments stored are valid when stored by the database.
    let commitment = CompressedPoint(rct_output.commitment);

    let key = CompressedPoint(rct_output.key);

    let txid = if get_txid {
        let txid = get_tx_from_id(&rct_output.tx_idx, tapes)?.hash();

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
    id: &PreRctOutputId,
    get_txid: bool,
    tx_ro: &heed::RoTxn,
    tapes: &tapes::Reader<MmapFile>,
    rct_tape: &tapes::FixedSizedTapeSlice<RctOutput>
) -> DbResult<OutputOnChain> {
    // v2 transactions.
    if id.amount == 0 {
        let rct_output = rct_tape
            .get(id.amount_index as usize)
            .ok_or(BlockchainError::NotFound)?;
        let output_on_chain = rct_output_to_output_on_chain(rct_output, get_txid, tapes)?;

        Ok(output_on_chain)
    } else {
        // v1 transactions.
        let output = get_output(id, tx_ro)?;
        let output_on_chain = output_to_output_on_chain(&output, id.amount, get_txid, tapes)?;

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
