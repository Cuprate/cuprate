//! General free functions (related to the database).

//---------------------------------------------------------------------------------------------------- Import

use cuprate_helper::map::u64_to_timelock;
use cuprate_types::OutputOnChain;
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, edwards::CompressedEdwardsY, Scalar};
use monero_serai::{transaction::Timelock, H};

use crate::{
    tables::{Tables, TxUnlockTime},
    types::{Amount, Output, OutputFlags},
    DatabaseRo, RuntimeError,
};

//---------------------------------------------------------------------------------------------------- Free functions
/// Map a [`crate::types::Output`] to a [`cuprate_types::OutputOnChain`].
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

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
