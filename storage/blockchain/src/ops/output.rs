//! Output functions.

//---------------------------------------------------------------------------------------------------- Import
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, edwards::CompressedEdwardsY, Scalar};
use monero_serai::{generators::H, transaction::Timelock};

use cuprate_database::{
    RuntimeError, {DatabaseRo, DatabaseRw},
};
use cuprate_helper::map::u64_to_timelock;
use cuprate_types::OutputOnChain;

use crate::{
    ops::macros::{doc_add_block_inner_invariant, doc_error},
    tables::{Outputs, RctOutputs, Tables, TablesMut, TxUnlockTime},
    types::{Amount, AmountIndex, Output, OutputFlags, PreRctOutputId, RctOutput},
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

//---------------------------------------------------------------------------------------------------- Mapping functions
/// Map an [`Output`] to a [`cuprate_types::OutputOnChain`].
#[doc = doc_error!()]
pub fn output_to_output_on_chain(
    output: &Output,
    amount: Amount,
    table_tx_unlock_time: &impl DatabaseRo<TxUnlockTime>,
) -> Result<OutputOnChain, RuntimeError> {
    // FIXME: implement lookup table for common values:
    // <https://github.com/monero-project/monero/blob/c8214782fb2a769c57382a999eaf099691c836e7/src/ringct/rctOps.cpp#L322>
    let commitment = ED25519_BASEPOINT_POINT + *H * Scalar::from(amount);

    let time_lock = if output
        .output_flags
        .contains(OutputFlags::NON_ZERO_UNLOCK_TIME)
    {
        u64_to_timelock(table_tx_unlock_time.get(&output.tx_idx)?)
    } else {
        Timelock::None
    };

    let key = CompressedEdwardsY::from_slice(&output.key)
        .map(|y| y.decompress())
        .unwrap_or(None);

    Ok(OutputOnChain {
        height: output.height as usize,
        time_lock,
        key,
        commitment,
    })
}

/// Map an [`RctOutput`] to a [`cuprate_types::OutputOnChain`].
///
/// # Panics
/// This function will panic if `rct_output`'s `commitment` fails to decompress
/// into a valid [`EdwardsPoint`](curve25519_dalek::edwards::EdwardsPoint).
///
/// This should normally not happen as commitments that
/// are stored in the database should always be valid.
#[doc = doc_error!()]
pub fn rct_output_to_output_on_chain(
    rct_output: &RctOutput,
    table_tx_unlock_time: &impl DatabaseRo<TxUnlockTime>,
) -> Result<OutputOnChain, RuntimeError> {
    // INVARIANT: Commitments stored are valid when stored by the database.
    let commitment = CompressedEdwardsY::from_slice(&rct_output.commitment)
        .unwrap()
        .decompress()
        .unwrap();

    let time_lock = if rct_output
        .output_flags
        .contains(OutputFlags::NON_ZERO_UNLOCK_TIME)
    {
        u64_to_timelock(table_tx_unlock_time.get(&rct_output.tx_idx)?)
    } else {
        Timelock::None
    };

    let key = CompressedEdwardsY::from_slice(&rct_output.key)
        .map(|y| y.decompress())
        .unwrap_or(None);

    Ok(OutputOnChain {
        height: rct_output.height as usize,
        time_lock,
        key,
        commitment,
    })
}

/// Map an [`PreRctOutputId`] to an [`OutputOnChain`].
///
/// Note that this still support RCT outputs, in that case, [`PreRctOutputId::amount`] should be `0`.
#[doc = doc_error!()]
pub fn id_to_output_on_chain(
    id: &PreRctOutputId,
    tables: &impl Tables,
) -> Result<OutputOnChain, RuntimeError> {
    // v2 transactions.
    if id.amount == 0 {
        let rct_output = get_rct_output(&id.amount_index, tables.rct_outputs())?;
        let output_on_chain = rct_output_to_output_on_chain(&rct_output, tables.tx_unlock_time())?;

        Ok(output_on_chain)
    } else {
        // v1 transactions.
        let output = get_output(id, tables.outputs())?;
        let output_on_chain =
            output_to_output_on_chain(&output, id.amount, tables.tx_unlock_time())?;

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
        open_tables::OpenTables,
        tables::{Tables, TablesMut},
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
                block_blobs: 0,
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
