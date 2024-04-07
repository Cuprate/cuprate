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
        BlockBlobs, BlockHeights, BlockInfoV1s, BlockInfoV2s, BlockInfoV3s, KeyImages, NumOutputs,
        Outputs, PrunableHashes, PrunableTxBlobs, PrunedTxBlobs, RctOutputs, Tables, TablesMut,
        TxHeights, TxIds, TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        Amount, AmountIndex, BlockHash, BlockHeight, BlockInfoLatest, BlockInfoV1, BlockInfoV2,
        BlockInfoV3, KeyImage, Output, PreRctOutputId, RctOutput, TxHash,
    },
};

//---------------------------------------------------------------------------------------------------- `add_output()`
/// TODO
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
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn add_output(
    amount: Amount,
    amount_index: AmountIndex,
    output: &Output,
    table_key_images: &mut impl DatabaseRw<KeyImages>,
    table_num_outputs: &mut impl DatabaseRw<NumOutputs>,
    table_outputs: &mut impl DatabaseRw<Outputs>,
) -> Result<(), RuntimeError> {
    todo!()
    // tables.num_outputs_mut().put(&amount, &amount_index)?;
    // tables.outputs_mut().put(&pre_rct_output_id, &output)?;
}

//---------------------------------------------------------------------------------------------------- `remove_output()`
/// TODO
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
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn remove_output(
    table_key_images: &mut impl DatabaseRw<KeyImages>,
    table_num_outputs: &mut impl DatabaseRw<NumOutputs>,
    table_outputs: &mut impl DatabaseRw<Outputs>,
) -> Result<(), RuntimeError> {
    todo!()
}

//---------------------------------------------------------------------------------------------------- `add_rct_output()`
/// TODO
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
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn add_rct_output(
    amount: Amount,
    amount_index: AmountIndex,
    rct_output: &RctOutput,
    table_key_images: &mut impl DatabaseRw<KeyImages>,
    table_num_outputs: &mut impl DatabaseRw<NumOutputs>,
    table_rct_outputs: &mut impl DatabaseRw<RctOutputs>,
) -> Result<(), RuntimeError> {
    // tables.num_outputs_mut().put(&amount, &amount_index)?;
    todo!()
}

//---------------------------------------------------------------------------------------------------- `remove_rct_output()`
/// TODO
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
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub fn remove_rct_output(
    amount: Amount,
    amount_index: AmountIndex,
    table_key_images: &mut impl DatabaseRw<KeyImages>,
    table_num_outputs: &mut impl DatabaseRw<NumOutputs>,
    table_rct_outputs: &mut impl DatabaseRw<RctOutputs>,
) -> Result<(), RuntimeError> {
    todo!()
}

//---------------------------------------------------------------------------------------------------- `get_output_*`
/// TODO
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_output(
    table_outputs: &(impl DatabaseRo<Outputs> + DatabaseIter<Outputs>),
    table_key_images: &(impl DatabaseRo<KeyImages> + DatabaseIter<KeyImages>),
    table_num_outputs: &(impl DatabaseRo<NumOutputs> + DatabaseIter<NumOutputs>),
    table_rct_outputs: &(impl DatabaseRo<RctOutputs> + DatabaseIter<RctOutputs>),
    amount: Amount,
    amount_index: AmountIndex,
) -> Result<OutputOnChain, RuntimeError> {
    todo!()
}

//----------------------------------------------------------------------------------------------------
/// TODO
pub fn get_output_list() {
    todo!()
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
    // TODO: is this correct?
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
pub fn get_num_outputs(
    table_outputs: &impl DatabaseRo<Outputs>,
) -> Result<u64, RuntimeError> {
    // TODO: is this correct?
    table_outputs.len()
}
