//! General free functions (related to the database).

//---------------------------------------------------------------------------------------------------- Import

use cuprate_helper::map::u64_to_timelock;
use cuprate_types::OutputOnChain;
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, edwards::CompressedEdwardsY, Scalar};
use monero_serai::{transaction::Timelock, Commitment, H};

use crate::{
    ops::output::{get_output, get_rct_output},
    tables::{Tables, TxUnlockTime},
    types::{Amount, Output, OutputFlags, PreRctOutputId, RctOutput},
    DatabaseRo, RuntimeError,
};

//---------------------------------------------------------------------------------------------------- Free functions
/// Map an [`Output`] to a [`cuprate_types::OutputOnChain`].
#[inline]
pub(crate) fn output_to_output_on_chain(
    output: &Output,
    amount: Amount,
    table_tx_unlock_time: &impl DatabaseRo<TxUnlockTime>,
) -> Result<OutputOnChain, RuntimeError> {
    // FIXME: implement lookup table for common values:
    // <https://github.com/monero-project/monero/blob/c8214782fb2a769c57382a999eaf099691c836e7/src/ringct/rctOps.cpp#L322>
    let commitment = ED25519_BASEPOINT_POINT + H() * Scalar::from(amount);

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
        height: u64::from(output.height),
        time_lock,
        key,
        commitment,
    })
}

/// Map an [`RctOutput`] to a [`cuprate_types::OutputOnChain`].
///
/// # Panics
/// This function will panic if `rct_output`'s `commitment` fails to decompress into a valid [`EdwardsPoint`].
#[inline]
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn rct_output_to_output_on_chain(
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
        height: u64::from(rct_output.height),
        time_lock,
        key,
        commitment,
    })
}

/// Map an [`PreRctOutputId`] to an [`OutputOnChain`].
///
/// Note that this still support RCT outputs, in that case, [`PreRctOutputId::amount`] should be `0`.
pub(crate) fn id_to_output_on_chain(
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
    // use super::*;
}
