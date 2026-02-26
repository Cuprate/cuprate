//! Output functions.

use fjall::Readable;
use std::collections::HashMap;
use std::io;
use std::ops::{AddAssign, SubAssign};
//---------------------------------------------------------------------------------------------------- Import
use cuprate_helper::{cast::u32_to_usize, crypto::compute_zero_commitment, map::u64_to_timelock};
use cuprate_types::OutputOnChain;
use monero_oxide::{io::CompressedPoint, transaction::Timelock};
use tapes::{TapesAppend, TapesRead, TapesTruncate};

use crate::error::{BlockchainError, DbResult};
use crate::types::TxId;
use crate::BlockchainDatabase;
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
    db: &BlockchainDatabase,
    amount: Amount,
    output: &Output,
    w: &mut fjall::OwnedWriteBatch,
    pre_rct_numb_outputs_cache: &mut HashMap<Amount, u64>,
) -> DbResult<PreRctOutputId> {
    let num_outputs = pre_rct_numb_outputs_cache.entry(amount).or_insert_with(|| {
        let last_out = db.pre_rct_outputs.prefix(amount.to_be_bytes()).next_back();

        last_out.map_or(0, |o| {
            u64::from_be_bytes(o.key().expect("TODO")[8..].try_into().unwrap()) + 1
        })
    });

    let pre_rct_output_id = PreRctOutputId {
        amount,
        // The new `amount_index` is the length of amount of outputs with same amount.
        amount_index: *num_outputs,
    };

    w.insert(
        &db.pre_rct_outputs,
        pre_rct_output_id_to_bytes(&pre_rct_output_id),
        bytemuck::bytes_of(output),
    );

    num_outputs.add_assign(1);

    Ok(pre_rct_output_id)
}

/// Remove a Pre-RCT [`Output`] from the database.
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
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

        last_out.map_or(0, |o| {
            u64::from_be_bytes(
                o.key().unwrap_or_else(|e| {
                    err = Some(e);
                    [0; 16].into()
                })[8..]
                    .try_into()
                    .unwrap(),
            ) + 1
        })
    });

    if let Some(e) = err {
        return Err(e.into());
    }

    let pre_rct_output_id = PreRctOutputId {
        amount,
        // The new `amount_index` is the length of amount of outputs with same amount.
        amount_index: *num_outputs,
    };

    tx_rw.remove(
        &db.pre_rct_outputs,
        pre_rct_output_id_to_bytes(&pre_rct_output_id),
    );

    num_outputs.sub_assign(1);
    Ok(())
}

/// Retrieve a Pre-RCT [`Output`] from the database.
#[doc = doc_error!()]
#[inline]
pub fn get_output(
    db: &BlockchainDatabase,
    pre_rct_output_id: &PreRctOutputId,
    tx_ro: &fjall::Snapshot,
) -> DbResult<Output> {
    let output = tx_ro
        .get(
            &db.pre_rct_outputs,
            pre_rct_output_id_to_bytes(pre_rct_output_id),
        )
        .expect("TODO")
        .ok_or(BlockchainError::NotFound)?;

    Ok(bytemuck::pod_read_unaligned(output.as_ref()))
}

/// How many pre-RCT [`Output`]s are there?
///
/// This returns the amount of pre-RCT outputs currently stored.
#[doc = doc_error!()]
#[inline]
pub fn get_num_outputs(db: &BlockchainDatabase, tx_ro: &fjall::Snapshot) -> DbResult<u64> {
    Ok(tx_ro.len(&db.pre_rct_outputs).expect("TODO") as u64)
}

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

//-xe--------------------------------------------------------- Mapping functions
/// Map an [`Output`] to a [`cuprate_types::OutputOnChain`].
#[doc = doc_error!()]
pub fn output_to_output_on_chain(
    output: &Output,
    amount: Amount,
    get_txid: bool,
    tapes: &tapes::TapesReadTransaction,
    db: &BlockchainDatabase,
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
    db: &BlockchainDatabase,
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
