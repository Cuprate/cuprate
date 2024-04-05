//! Outputs.

//---------------------------------------------------------------------------------------------------- Import
use monero_serai::transaction::{Timelock, Transaction};

use cuprate_types::{OutputOnChain, VerifiedBlockInformation};

use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::{
        blockchain::height_internal,
        macros::{doc_error, doc_fn},
    },
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

//---------------------------------------------------------------------------------------------------- Private
/// TODO
#[inline]
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub(super) fn add_output(
    table_outputs: &mut impl DatabaseRw<Outputs>,
    table_key_images: &mut impl DatabaseRw<KeyImages>,
    table_num_outputs: &mut impl DatabaseRw<NumOutputs>,
    table_rct_outputs: &mut impl DatabaseRw<RctOutputs>,
) {
    todo!()
}

/// TODO
#[inline]
#[allow(clippy::needless_pass_by_ref_mut)] // TODO: remove me
pub(super) fn remove_output(
    table_outputs: &mut impl DatabaseRw<Outputs>,
    table_key_images: &mut impl DatabaseRw<KeyImages>,
    table_num_outputs: &mut impl DatabaseRw<NumOutputs>,
    table_rct_outputs: &mut impl DatabaseRw<RctOutputs>,
) {
    todo!()
}

//---------------------------------------------------------------------------------------------------- `get_output_*`
/// TODO
///
#[doc = doc_fn!(get_output_bulk)]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn get_output<'env, Ro, Rw, Env>(
    env: &Env,
    tx_ro: &Ro,
    amount: Amount,
    amount_index: AmountIndex,
) -> Result<OutputOnChain, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
{
    get_output_internal(
        amount,
        amount_index,
        &env.open_db_ro::<Outputs>(tx_ro)?,
        &env.open_db_ro::<KeyImages>(tx_ro)?,
        &env.open_db_ro::<NumOutputs>(tx_ro)?,
        &env.open_db_ro::<RctOutputs>(tx_ro)?,
    )
}

/// TODO
///
#[doc = doc_fn!(get_output, bulk)]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::output::*};
/// // TODO
/// ```
#[doc = doc_error!(bulk)]
#[inline]
pub fn get_output_bulk<'env, Ro, Rw, Env, Iter>(
    env: &Env,
    tx_ro: &Ro,
    outputs: Iter,
) -> Result<Vec<OutputOnChain>, RuntimeError>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Env: EnvInner<'env, Ro, Rw>,
    Iter: Iterator<Item = (Amount, AmountIndex)> + ExactSizeIterator,
{
    let (table_outputs, table_key_images, table_num_outputs, table_rct_outputs) = (
        &env.open_db_ro::<Outputs>(tx_ro)?,
        &env.open_db_ro::<KeyImages>(tx_ro)?,
        &env.open_db_ro::<NumOutputs>(tx_ro)?,
        &env.open_db_ro::<RctOutputs>(tx_ro)?,
    );

    let mut vec = Vec::with_capacity(outputs.len());

    for (amount, amount_index) in outputs {
        let output = get_output_internal(
            amount,
            amount_index,
            table_outputs,
            table_key_images,
            table_num_outputs,
            table_rct_outputs,
        )?;
        vec.push(output);
    }

    Ok(vec)
}

/// TODO
#[inline]
pub(super) fn get_output_internal(
    amount: Amount,
    amount_index: AmountIndex,
    table_outputs: &(impl DatabaseRo<Outputs> + DatabaseIter<Outputs>),
    table_key_images: &(impl DatabaseRo<KeyImages> + DatabaseIter<KeyImages>),
    table_num_outputs: &(impl DatabaseRo<NumOutputs> + DatabaseIter<NumOutputs>),
    table_rct_outputs: &(impl DatabaseRo<RctOutputs> + DatabaseIter<RctOutputs>),
) -> Result<OutputOnChain, RuntimeError> {
    todo!()
}

//----------------------------------------------------------------------------------------------------
/// TODO
pub fn get_output_list() {
    todo!()
}

/// TODO
pub fn get_rct_num_outputs() {
    todo!()
}

/// TODO
pub fn get_pre_rct_num_outputs() {
    todo!()
}
