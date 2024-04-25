//! Outputs.

//---------------------------------------------------------------------------------------------------- Import
use monero_serai::transaction::{Timelock, Transaction};

use cuprate_types::{OutputOnChain, VerifiedBlockInformation};

use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::macros::{doc_add_block_inner_invariant, doc_error},
    tables::{
        BlockBlobs, BlockHeights, BlockInfos, KeyImages, NumOutputs, Outputs, PrunableHashes,
        PrunableTxBlobs, PrunedTxBlobs, RctOutputs, Tables, TablesMut, TxHeights, TxIds,
        TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        Amount, AmountIndex, BlockHash, BlockHeight, BlockInfo, KeyImage, Output, PreRctOutputId,
        RctOutput, TxHash,
    },
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
    output: &Output,
    tables: &mut impl TablesMut,
) -> Result<PreRctOutputId, RuntimeError> {
    // FIXME: this would be much better expressed with a
    // `btree_map::Entry`-like API, fix `trait DatabaseRw`.
    let num_outputs = match tables.num_outputs().get(&amount) {
        // Entry with `amount` already exists.
        Ok(num_outputs) => num_outputs,
        // Entry with `amount` didn't exist, this is
        // the 1st output with this amount.
        Err(RuntimeError::KeyNotFound) => 0,
        Err(e) => return Err(e),
    };
    // Update the amount of outputs.
    tables.num_outputs_mut().put(&amount, &(num_outputs + 1))?;

    let pre_rct_output_id = PreRctOutputId {
        amount,
        // The new `amount_index` is the length of amount of outputs with same amount.
        amount_index: num_outputs,
    };

    tables.outputs_mut().put(&pre_rct_output_id, output)?;
    Ok(pre_rct_output_id)
}

/// Remove a Pre-RCT [`Output`] from the database.
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
#[inline]
pub fn remove_output(
    pre_rct_output_id: &PreRctOutputId,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    // Decrement the amount index by 1, or delete the entry out-right.
    // FIXME: this would be much better expressed with a
    // `btree_map::Entry`-like API, fix `trait DatabaseRw`.
    tables
        .num_outputs_mut()
        .update(&pre_rct_output_id.amount, |num_outputs| {
            // INVARIANT: Should never be 0.
            if num_outputs == 1 {
                None
            } else {
                Some(num_outputs - 1)
            }
        })?;

    // Delete the output data itself.
    tables.outputs_mut().delete(pre_rct_output_id)
}

/// Retrieve a Pre-RCT [`Output`] from the database.
#[doc = doc_error!()]
#[inline]
pub fn get_output(
    pre_rct_output_id: &PreRctOutputId,
    table_outputs: &impl DatabaseRo<Outputs>,
) -> Result<Output, RuntimeError> {
    table_outputs.get(pre_rct_output_id)
}

/// How many pre-RCT [`Output`]s are there?
///
/// This returns the amount of pre-RCT outputs currently stored.
#[doc = doc_error!()]
#[inline]
pub fn get_num_outputs(table_outputs: &impl DatabaseRo<Outputs>) -> Result<u64, RuntimeError> {
    table_outputs.len()
}

//---------------------------------------------------------------------------------------------------- RCT Outputs
/// Add an [`RctOutput`] to the database.
///
/// Upon [`Ok`], this function returns the [`AmountIndex`] that
/// can be used to lookup the `RctOutput` in [`get_rct_output()`].
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
#[inline]
pub fn add_rct_output(
    rct_output: &RctOutput,
    table_rct_outputs: &mut impl DatabaseRw<RctOutputs>,
) -> Result<AmountIndex, RuntimeError> {
    let amount_index = get_rct_num_outputs(table_rct_outputs)?;
    table_rct_outputs.put(&amount_index, rct_output)?;
    Ok(amount_index)
}

/// Remove an [`RctOutput`] from the database.
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
#[inline]
pub fn remove_rct_output(
    amount_index: &AmountIndex,
    table_rct_outputs: &mut impl DatabaseRw<RctOutputs>,
) -> Result<(), RuntimeError> {
    table_rct_outputs.delete(amount_index)
}

/// Retrieve an [`RctOutput`] from the database.
#[doc = doc_error!()]
#[inline]
pub fn get_rct_output(
    amount_index: &AmountIndex,
    table_rct_outputs: &impl DatabaseRo<RctOutputs>,
) -> Result<RctOutput, RuntimeError> {
    table_rct_outputs.get(amount_index)
}

/// How many [`RctOutput`]s are there?
///
/// This returns the amount of RCT outputs currently stored.
#[doc = doc_error!()]
#[inline]
pub fn get_rct_num_outputs(
    table_rct_outputs: &impl DatabaseRo<RctOutputs>,
) -> Result<u64, RuntimeError> {
    table_rct_outputs.len()
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
#[allow(clippy::significant_drop_tightening, clippy::cognitive_complexity)]
mod test {
    use super::*;
    use crate::{
        tests::{assert_all_tables_are_empty, tmp_concrete_env},
        types::OutputFlags,
        Env,
    };
    use cuprate_test_utils::data::{tx_v1_sig2, tx_v2_rct3};
    use pretty_assertions::assert_eq;

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
        let (env, tmp) = tmp_concrete_env();
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
            assert_eq!(tables.block_infos().len().unwrap(), 0);
            assert_eq!(tables.block_blobs().len().unwrap(), 0);
            assert_eq!(tables.block_heights().len().unwrap(), 0);
            assert_eq!(tables.key_images().len().unwrap(), 0);
            assert_eq!(tables.num_outputs().len().unwrap(), 1);
            assert_eq!(tables.pruned_tx_blobs().len().unwrap(), 0);
            assert_eq!(tables.prunable_hashes().len().unwrap(), 0);
            assert_eq!(tables.outputs().len().unwrap(), 1);
            assert_eq!(tables.prunable_tx_blobs().len().unwrap(), 0);
            assert_eq!(tables.rct_outputs().len().unwrap(), 1);
            assert_eq!(tables.tx_blobs().len().unwrap(), 0);
            assert_eq!(tables.tx_ids().len().unwrap(), 0);
            assert_eq!(tables.tx_heights().len().unwrap(), 0);
            assert_eq!(tables.tx_unlock_time().len().unwrap(), 0);

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
