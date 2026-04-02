//! Output functions.
use std::{
    collections::HashMap,
    ops::{AddAssign, SubAssign},
};

use fjall::Readable;
use monero_oxide::ed25519::CompressedPoint;
use tapes::TapesRead;

use cuprate_helper::{crypto::compute_zero_commitment, map::u64_to_timelock};
use cuprate_types::OutputOnChain;

use crate::{
    error::{BlockchainError, DbResult},
    ops::tx::get_tx_from_id,
    types::{Amount, Output, PreRctOutputId, RctOutput},
    BlockchainDatabase,
};

/// Add a Pre-RCT [`Output`] to the database.
///
#[inline]
pub fn add_output(
    db: &BlockchainDatabase,
    amount: Amount,
    output: &Output,
    w: &mut fjall::OwnedWriteBatch,
    pre_rct_numb_outputs_cache: &mut HashMap<Amount, u64>,
) -> DbResult<PreRctOutputId> {
    let mut err = None;
    let num_outputs = pre_rct_numb_outputs_cache.entry(amount).or_insert_with(|| {
        let last_out = db.pre_rct_outputs.prefix(amount.to_be_bytes()).next_back();

        match last_out.map(fjall::Guard::key) {
            None => 0,
            Some(Ok(o)) => u64::from_be_bytes(o[8..].try_into().unwrap()) + 1,
            Some(Err(e)) => {
                err = Some(e);
                0
            }
        }
    });

    if let Some(e) = err {
        return Err(e.into());
    }

    let pre_rct_output_id = PreRctOutputId {
        amount,
        // The new `amount_index` is the length of amount of outputs with same amount.
        amount_index: *num_outputs,
    };

    w.insert(
        &db.pre_rct_outputs,
        pre_rct_output_id.to_bytes(),
        bytemuck::bytes_of(output),
    );

    num_outputs.add_assign(1);

    Ok(pre_rct_output_id)
}

/// Remove a Pre-RCT [`Output`] from the database.
#[inline]
pub fn remove_output(
    db: &BlockchainDatabase,
    amount: Amount,
    tx_rw: &mut fjall::OwnedWriteBatch,
) -> DbResult<()> {
    let mut pre_rct_numb_outputs_cache = db.pre_rct_numb_outputs_cache.lock().unwrap();

    let mut err = None;
    let num_outputs = pre_rct_numb_outputs_cache.entry(amount).or_insert_with(|| {
        let last_out = db.pre_rct_outputs.prefix(amount.to_be_bytes()).next_back();

        match last_out.map(fjall::Guard::key) {
            None => 0,
            Some(Ok(o)) => u64::from_be_bytes(o[8..].try_into().unwrap()) + 1,
            Some(Err(e)) => {
                err = Some(e);
                0
            }
        }
    });

    if let Some(e) = err {
        return Err(e.into());
    }

    let pre_rct_output_id = PreRctOutputId {
        amount,
        // The new `amount_index` is the length of amount of outputs with same amount.
        amount_index: *num_outputs,
    };

    tx_rw.remove(&db.pre_rct_outputs, pre_rct_output_id.to_bytes());

    num_outputs.sub_assign(1);
    Ok(())
}

/// Retrieve a Pre-RCT [`Output`] from the database.
#[inline]
pub fn get_output(
    db: &BlockchainDatabase,
    pre_rct_output_id: &PreRctOutputId,
    tx_ro: &fjall::Snapshot,
) -> DbResult<Output> {
    let output = tx_ro
        .get(&db.pre_rct_outputs, pre_rct_output_id.to_bytes())?
        .ok_or(BlockchainError::NotFound)?;

    Ok(bytemuck::pod_read_unaligned(output.as_ref()))
}

/// Get the number of outputs with a given amount.
#[inline]
pub fn get_num_outputs_with_amount(
    db: &BlockchainDatabase,
    tx_ro: &fjall::Snapshot,
    amount: Amount,
) -> DbResult<u64> {
    let last_out = tx_ro
        .prefix(&db.pre_rct_outputs, amount.to_be_bytes())
        .next_back();

    last_out.map_or(Ok(0), |o| {
        Ok(u64::from_be_bytes(o.key()?[8..].try_into().unwrap()) + 1)
    })
}

/// Map an [`Output`] to a [`cuprate_types::OutputOnChain`].
pub fn output_to_output_on_chain(
    output: &Output,
    amount: Amount,
    get_txid: bool,
    tapes: &tapes::TapesReadTransaction,
    db: &BlockchainDatabase,
) -> DbResult<OutputOnChain> {
    let commitment = compute_zero_commitment(amount);

    let key = CompressedPoint::from(output.key);

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

/// Map a [`RctOutput`] to a [`cuprate_types::OutputOnChain`].
#[inline]
pub fn rct_output_to_output_on_chain(
    rct_output: &RctOutput,
    get_txid: bool,
    tapes: &tapes::TapesReadTransaction,
    db: &BlockchainDatabase,
) -> DbResult<OutputOnChain> {
    // INVARIANT: Commitments stored are valid when stored by the database.
    let commitment = CompressedPoint::from(rct_output.commitment);

    let key = CompressedPoint::from(rct_output.key);

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
pub fn id_to_output_on_chain(
    db: &BlockchainDatabase,
    id: &PreRctOutputId,
    get_txid: bool,
    tx_ro: &fjall::Snapshot,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<OutputOnChain> {
    // v2 transactions.
    if id.amount == 0 {
        let rct_output = tapes
            .read_entry(&db.rct_outputs, id.amount_index)?
            .ok_or(BlockchainError::NotFound)?;
        let output_on_chain = rct_output_to_output_on_chain(&rct_output, get_txid, tapes, db)?;

        Ok(output_on_chain)
    } else {
        // v1 transactions.
        let output = get_output(db, id, tx_ro)?;
        let output_on_chain = output_to_output_on_chain(&output, id.amount, get_txid, tapes, db)?;

        Ok(output_on_chain)
    }
}
