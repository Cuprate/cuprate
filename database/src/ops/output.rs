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

//---------------------------------------------------------------------------------------------------- `add_output()`
/// Add a Pre-RCT [`Output`] to the database.
///
/// Upon [`Ok`], this function returns the [`AmountIndex`] that
/// can be used to lookup the `Output` in [`get_output()`].
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[inline]
pub fn add_output(
    amount: Amount,
    output: &Output,
    tables: &mut impl TablesMut,
    // table_outputs: &mut impl DatabaseRw<Outputs>,
    // table_num_outputs: &mut impl DatabaseRw<NumOutputs>,
) -> Result<AmountIndex, RuntimeError> {
    let amount_index = get_num_outputs(tables.outputs_mut())?;
    tables.num_outputs_mut().put(&amount, &amount_index)?;

    let pre_rct_output_id = PreRctOutputId {
        amount,
        amount_index,
    };

    tables.outputs_mut().put(&pre_rct_output_id, output)?;
    Ok(amount_index)
}

//---------------------------------------------------------------------------------------------------- `remove_output()`
/// Remove a Pre-RCT [`Output`] from the database.
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[inline]
pub fn remove_output(
    pre_rct_output_id: &PreRctOutputId,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    // Decrement the amount index by 1, or delete the entry out-right.
    tables
        .num_outputs_mut()
        .update(&pre_rct_output_id.amount, |amount| {
            if amount == 0 {
                None
            } else {
                Some(amount - 1)
            }
        })?;

    // Delete the output data itself.
    tables.outputs_mut().delete(pre_rct_output_id)
}

//---------------------------------------------------------------------------------------------------- `add_rct_output()`
/// Add an [`RctOutput`] to the database.
///
/// Upon [`Ok`], this function returns the [`AmountIndex`] that
/// can be used to lookup the `RctOutput` in [`get_rct_output()`].
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[inline]
pub fn add_rct_output(
    rct_output: &RctOutput,
    table_rct_outputs: &mut impl DatabaseRw<RctOutputs>,
) -> Result<AmountIndex, RuntimeError> {
    let amount_index = get_rct_num_outputs(table_rct_outputs)?;
    table_rct_outputs.put(&amount_index, rct_output)?;
    Ok(amount_index)
}

//---------------------------------------------------------------------------------------------------- `remove_rct_output()`
/// Remove an [`RctOutput`] from the database.
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[inline]
pub fn remove_rct_output(
    amount_index: &AmountIndex,
    table_rct_outputs: &mut impl DatabaseRw<RctOutputs>,
) -> Result<(), RuntimeError> {
    table_rct_outputs.delete(amount_index)
}

//---------------------------------------------------------------------------------------------------- `get_output_*`
/// Retrieve a Pre-RCT [`Output`] from the database.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_output(
    pre_rct_output_id: &PreRctOutputId,
    table_outputs: &impl DatabaseRo<Outputs>,
) -> Result<Output, RuntimeError> {
    table_outputs.get(pre_rct_output_id)
}

/// Retrieve an [`RctOutput`] from the database.
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_rct_output(
    amount_index: &AmountIndex,
    table_rct_outputs: &impl DatabaseRo<RctOutputs>,
) -> Result<RctOutput, RuntimeError> {
    table_rct_outputs.get(amount_index)
}

//---------------------------------------------------------------------------------------------------- `get_rct_num_outputs()`
/// TODO
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_rct_num_outputs(
    table_rct_outputs: &impl DatabaseRo<RctOutputs>,
) -> Result<u64, RuntimeError> {
    table_rct_outputs.len()
}

//---------------------------------------------------------------------------------------------------- `get_num_outputs()`
/// TODO
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_num_outputs(table_outputs: &impl DatabaseRo<Outputs>) -> Result<u64, RuntimeError> {
    table_outputs.len()
}
