//! Block functions.

use std::borrow::Cow;
use std::cmp::min;
use std::collections::HashMap;
use std::io;
//---------------------------------------------------------------------------------------------------- Import
use std::io::Write;

use crate::error::{BlockchainError, DbResult};
use crate::ops::tx::{add_tx_to_dynamic_tables, add_tx_to_tapes, remove_tx_from_dynamic_tables};
use crate::types::{Amount, RctOutput, TxInfo};
use crate::BlockchainDatabase;
use crate::{
    ops::{alt_block, blockchain::chain_height, macros::doc_error},
    types::{BlockHash, BlockHeight, BlockInfo},
};
use bytemuck::TransparentWrapper;
use bytes::Bytes;
use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_helper::{
    map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits},
    tx::tx_fee,
};
use cuprate_pruning::CRYPTONOTE_PRUNING_LOG_STRIPES;
use cuprate_types::{
    AltBlockInformation, BlockCompleteEntry, ChainId, ExtendedBlockHeader, HardFork,
    PrunedTxBlobEntry, TransactionBlobs, VerifiedBlockInformation, VerifiedTransactionInformation,
};
use fjall::Readable;
use monero_oxide::transaction::Pruned;
use monero_oxide::{
    block::{Block, BlockHeader},
    transaction::Transaction,
};
use tapes::{TapesAppend, TapesRead, TapesTruncate};
use tracing::instrument;

#[instrument(skip_all, level = "info")]
pub fn add_blocks_to_tapes(
    blocks: &[VerifiedBlockInformation],
    db: &BlockchainDatabase,
    append_tx: &mut tapes::TapesAppendTransaction,
) -> DbResult<()> {
    let mut pruned_tape_index = append_tx.blob_tape_len(&db.pruned_blobs).unwrap_or(0);
    for block in blocks {
        append_tx.append_bytes(&db.pruned_blobs, &block.block_blob)?;

        for tx in &block.txs {
            append_tx.append_bytes(&db.pruned_blobs, tx.tx_pruned.as_slice())?;

            let prunable_hash = if tx.tx_prunable_blob.is_empty() || tx.tx.version() == 1 {
                [0; 32]
            } else {
                monero_oxide::primitives::keccak256(&tx.tx_prunable_blob)
            };
            append_tx.append_bytes(&db.pruned_blobs, &prunable_hash)?;
        }
    }

    tracing::trace!("pruned_tape_index: {}", pruned_tape_index);

    let mut write_v2_prunable_data = |append_tx: &mut tapes::TapesAppendTransaction,
                                      tape,
                                      blocks: &[VerifiedBlockInformation]|
     -> io::Result<u64> {
        let mut first_idx = append_tx
            .blob_tape_len(tape)
            .expect("Required tape not found");
        for block in blocks {
            for tx in &block.txs {
                if tx.tx.version() != 1 {
                    append_tx.append_bytes(tape, tx.tx_prunable_blob.as_slice())?;
                }
            }
        }

        Ok(first_idx)
    };

    let mut write_v1_prunable_data = |append_tx: &mut tapes::TapesAppendTransaction,
                                      blocks: &[VerifiedBlockInformation]|
     -> io::Result<u64> {
        let mut first_idx = u64::MAX;
        let mut first_idx = append_tx
            .blob_tape_len(&db.v1_prunable_blobs)
            .expect("Required tape not found");

        for block in blocks {
            for tx in &block.txs {
                if tx.tx.version() == 1 {
                    append_tx.append_bytes(&db.v1_prunable_blobs, &tx.tx_prunable_blob)?;
                }
            }
        }

        Ok(first_idx)
    };

    // Split the blocks at the point the pruning stripe changes.
    let start_height = blocks[0].height;
    let first_block_pruning_seed = cuprate_pruning::DecompressedPruningSeed::new(
        cuprate_pruning::get_block_pruning_stripe(start_height, usize::MAX, 3).unwrap(),
        3,
    )
    .unwrap();
    let next_stripe_height = first_block_pruning_seed
        .get_next_pruned_block(start_height, 500_000_000)
        .unwrap()
        .unwrap();

    let (first_stripe, next_stripe) =
        blocks.split_at(min(next_stripe_height - start_height, blocks.len()));

    tracing::debug!(
        start_height,
        ?first_block_pruning_seed,
        next_stripe_height,
        first_stripe_len = first_stripe.len(),
        next_stripe_len = next_stripe.len()
    );

    for blocks in [first_stripe, next_stripe] {
        if blocks.is_empty() {
            continue;
        }

        let stripe =
            cuprate_pruning::get_block_pruning_stripe(blocks[0].height, usize::MAX, 3).unwrap();
        let mut v2_prunable_index =
            write_v2_prunable_data(append_tx, &db.prunable_blobs[stripe as usize - 1], blocks)?;

        let mut v1_prunable_index = write_v1_prunable_data(append_tx, blocks)?;

        let mut numb_rct_outs = append_tx
            .fixed_sized_tape_len(&db.rct_outputs)
            .expect("Required tape not found");

        tracing::debug!(
            chunk_start = blocks[0].height,
            stripe,
            v1_prunable_index,
            v2_prunable_index,
            numb_rct_outs
        );

        for block in blocks {
            let block_pruned_blob_idx = pruned_tape_index;
            let block_v1_prunable_idx = v1_prunable_index;
            let block_v2_prunable_idx = v2_prunable_index;

            let header_len = block.block.header.serialize().len() as u64;

            let mining_tx_index = {
                let tx = block.block.miner_transaction();
                add_tx_to_tapes(
                    &tx.clone().into(),
                    pruned_tape_index + header_len,
                    0,
                    tx.serialize().len(),
                    0,
                    &block.height,
                    &mut numb_rct_outs,
                    append_tx,
                    db,
                )?
            };

            pruned_tape_index += block.block_blob.len() as u64;

            for tx in &block.txs {
                add_tx_to_tapes(
                    &tx.tx,
                    pruned_tape_index,
                    if tx.tx.version() == 1 {
                        v1_prunable_index
                    } else {
                        v2_prunable_index
                    },
                    tx.tx_pruned.len(),
                    tx.tx_prunable_blob.len(),
                    &block.height,
                    &mut numb_rct_outs,
                    append_tx,
                    db,
                )?;

                pruned_tape_index += tx.tx_pruned.len() as u64 + 32;
                if tx.tx.version() == 1 {
                    v1_prunable_index += tx.tx_prunable_blob.len() as u64;
                } else {
                    v2_prunable_index += tx.tx_prunable_blob.len() as u64;
                }
            }

            // `saturating_add` is used here as cumulative generated coins overflows due to tail emission.
            let cumulative_generated_coins = append_tx
                .read_entry(&db.block_infos, block.height.saturating_sub(1) as u64)?
                .map_or(0, |prev| prev.cumulative_generated_coins)
                .saturating_add(block.generated_coins);

            let (cumulative_difficulty_low, cumulative_difficulty_high) =
                split_u128_into_low_high_bits(block.cumulative_difficulty);

            append_tx.append_entries(
                &db.block_infos,
                &[BlockInfo {
                    cumulative_difficulty_low,
                    cumulative_difficulty_high,
                    cumulative_generated_coins,
                    cumulative_rct_outs: numb_rct_outs,
                    block_hash: block.block_hash,
                    weight: block.weight,
                    long_term_weight: block.long_term_weight,
                    mining_tx_index,
                    pruned_blob_idx: block_pruned_blob_idx,
                    v1_prunable_blob_idx: block_v1_prunable_idx,
                    prunable_blob_idx: block_v2_prunable_idx,
                }],
            )?;

            tracing::debug!(
                height = block.height,
                block_pruned_blob_idx,
                block_v1_prunable_idx,
                block_v2_prunable_idx,
                cumulative_generated_coins,
                "added block to tapes"
            );
        }
    }

    Ok(())
}

/// Add a [`VerifiedBlockInformation`] to the database.
///
/// This extracts all the data from the input block and
/// maps/adds them to the appropriate database tables.
///
#[doc = doc_error!()]
///
/// # Panics
/// This function will panic if:
/// - `block.height > u32::MAX` (not normally possible)
/// - `block.height` is != [`chain_height`]
#[expect(single_use_lifetimes)]
// no inline, too big.
pub fn add_block_to_dynamic_tables<'a>(
    db: &BlockchainDatabase,
    block: &Block,
    block_hash: &BlockHash,
    txs: impl Iterator<Item = Cow<'a, Transaction<Pruned>>>,
    numb_transactions: &mut u64,
    w: &mut fjall::OwnedWriteBatch,
    pre_rct_numb_outputs_cache: &mut HashMap<Amount, u64>,
) -> DbResult<()> {
    //------------------------------------------------------ Check preconditions first

    // Cast height to `u32` for storage (handled at top of function).
    // Panic (should never happen) instead of allowing DB corruption.
    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1560020991>
    assert!(
        u32::try_from(block.number()).is_ok(),
        "block.height ({}) > u32::MAX",
        block.number(),
    );

    //------------------------------------------------------ Transaction / Outputs / Key Images
    // Add the miner transaction first.
    let tx = block.miner_transaction();
    add_tx_to_dynamic_tables(
        db,
        &tx.clone().into(),
        *numb_transactions,
        &tx.hash(),
        &block.number(),
        w,
        pre_rct_numb_outputs_cache,
    )?;
    *numb_transactions += 1;

    for (tx, tx_hash) in txs.zip(&block.transactions) {
        add_tx_to_dynamic_tables(
            db,
            &tx,
            *numb_transactions,
            tx_hash,
            &block.number(),
            w,
            pre_rct_numb_outputs_cache,
        )?;
        *numb_transactions += 1;
    }

    w.insert(&db.block_heights, block_hash, block.number().to_le_bytes());

    Ok(())
}

//---------------------------------------------------------------------------------------------------- `pop_block`
/// TODO.
// no inline, too big
pub fn pop_block(
    db: &BlockchainDatabase,
    move_to_alt_chain: Option<ChainId>,
    tx_rw: &mut fjall::OwnedWriteBatch,
    tapes: &mut tapes::TapesTruncateTransaction,
) -> DbResult<(BlockHeight, BlockHash, Block)> {
    //------------------------------------------------------ Block Info
    // Remove block data from tables.
    let (block_height, block_info) = tapes
        .pop_fixed_sized_tape(&db.block_infos)?
        .ok_or(BlockchainError::NotFound)?;

    let block_height = usize::try_from(block_height).unwrap();

    tx_rw.remove(&db.block_heights, block_info.block_hash);

    // Block blobs.
    //
    // We deserialize the block header blob and mining transaction blob
    // to form a `Block`, such that we can remove the associated transactions
    // later.

    let block = get_block(&block_height, Some(&block_info), tapes, db)?;
    //------------------------------------------------------ Transaction / Outputs / Key Images
    remove_tx_from_dynamic_tables(
        db,
        &block.miner_transaction().hash(),
        block_height,
        tx_rw,
        tapes,
    )?;

    let remove_tx_iter = block.transactions.iter().map(|tx_hash| {
        let (_, tx) = remove_tx_from_dynamic_tables(db, tx_hash, block_height, tx_rw, tapes)?;
        Ok::<_, BlockchainError>(tx)
    });

    if let Some(chain_id) = move_to_alt_chain {
        let txs = remove_tx_iter
            .map(|result| {
                let tx = result?;
                let tx_weight = tx.weight();
                let tx_hash = tx.hash();
                let fee = tx_fee(&tx);
                let (tx_pruned, prunable) = tx.pruned_with_prunable();

                Ok(VerifiedTransactionInformation {
                    tx_weight,
                    tx_pruned: tx_pruned.serialize(),
                    tx_prunable_blob: prunable,
                    tx_hash,
                    fee,
                    tx: tx_pruned,
                })
            })
            .collect::<DbResult<Vec<VerifiedTransactionInformation>>>()?;

        alt_block::add_alt_block(
            db,
            &AltBlockInformation {
                block: block.clone(),
                block_blob: block.serialize(),
                txs,
                block_hash: block_info.block_hash,
                // We know the PoW is valid for this block so just set it so it will always verify as valid.
                pow_hash: [0; 32],
                height: block_height,
                weight: block_info.weight,
                long_term_weight: block_info.long_term_weight,
                cumulative_difficulty: combine_low_high_bits_to_u128(
                    block_info.cumulative_difficulty_low,
                    block_info.cumulative_difficulty_high,
                ),
                chain_id,
            },
            tx_rw,
        )?;
    } else {
        for result in remove_tx_iter {
            drop(result?);
        }
    }

    tapes.truncate_blob_tape(&db.pruned_blobs, block_info.pruned_blob_idx);
    tapes.truncate_blob_tape(&db.v1_prunable_blobs, block_info.v1_prunable_blob_idx);
    let stripe = cuprate_pruning::get_block_pruning_stripe(block_height, usize::MAX, 3).unwrap();
    tapes.truncate_blob_tape(
        &db.prunable_blobs[stripe as usize - 1],
        block_info.prunable_blob_idx,
    );

    tapes.truncate_fixed_sized_tape(&db.tx_infos, block_info.mining_tx_index);

    let cumulative_rct_outs = tapes
        .read_entry(&db.block_infos, block_height as u64 - 1)?
        .map_or(0, |info| info.cumulative_rct_outs);

    tapes.truncate_fixed_sized_tape(&db.rct_outputs, cumulative_rct_outs);

    Ok((block_height, block_info.block_hash, block))
}

//---------------------------------------------------------------------------------------------------- `get_block_complete_entry_*`
/// Retrieve a [`BlockCompleteEntry`] from the database.
///
#[doc = doc_error!()]
pub fn get_block_complete_entry(
    db: &BlockchainDatabase,
    block_hash: &BlockHash,
    pruned: bool,
    tx_ro: &fjall::Snapshot,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<BlockCompleteEntry> {
    let block_height = tx_ro
        .get(&db.block_heights, block_hash)?
        .ok_or(BlockchainError::NotFound)?;
    get_block_complete_entry_from_height(
        usize::from_le_bytes(block_height.as_ref().try_into().unwrap()),
        pruned,
        tapes,
        db,
    )
}

/// Retrieve a [`BlockCompleteEntry`] from the database.
///
#[doc = doc_error!()]
pub fn get_block_complete_entry_from_height(
    block_height: BlockHeight,
    pruned: bool,
    tapes: &tapes::TapesReadTransaction,
    db: &BlockchainDatabase,
) -> DbResult<BlockCompleteEntry> {
    let pruning_stripe = cuprate_pruning::get_block_pruning_stripe(
        block_height,
        usize::MAX,
        CRYPTONOTE_PRUNING_LOG_STRIPES,
    )
    .unwrap();

    let mut block_info = tapes
        .read_entry(&db.block_infos, block_height as u64)?
        .ok_or(BlockchainError::NotFound)?;

    let block_blob_start_idx = block_info.pruned_blob_idx;
    let mut block_blob_end_idx = None;

    let mut txs = Vec::with_capacity(32);

    for tx_info in tapes.iter_from(&db.tx_infos, block_info.mining_tx_index + 1)? {
        let tx_info = tx_info?;

        if tx_info.height != block_height {
            break;
        }

        block_blob_end_idx.get_or_insert(tx_info.pruned_blob_idx);

        txs.push(tx_info);
    }

    let txs = if txs.is_empty() {
        TransactionBlobs::None
    } else if pruned {
        let first_blob_idx = txs.first().unwrap().pruned_blob_idx;
        let mut blob = vec![
            0;
            u64_to_usize(txs.last().unwrap().pruned_blob_idx - first_blob_idx)
                + txs.last().unwrap().pruned_size
                + 32
        ];
        tapes.read_bytes(&db.pruned_blobs, first_blob_idx, &mut blob)?;

        let mut bytes = Bytes::from(blob);

        TransactionBlobs::Pruned(
            txs.into_iter()
                .map(|tx_info| PrunedTxBlobEntry {
                    blob: bytes.split_to(tx_info.pruned_size),
                    prunable_hash: bytes.split_to(32).try_into().unwrap(),
                })
                .collect(),
        )
    } else {
        TransactionBlobs::Normal(
            txs.into_iter()
                .map(|tx_info| {
                    let mut blob = vec![0; tx_info.pruned_size + tx_info.prunable_size];

                    tapes.read_bytes(
                        &db.pruned_blobs,
                        tx_info.pruned_blob_idx,
                        &mut blob[..tx_info.pruned_size],
                    )?;
                    if tx_info.rct_output_start_idx == u64::MAX {
                        tapes.read_bytes(
                            &db.v1_prunable_blobs,
                            tx_info.prunable_blob_idx,
                            &mut blob[tx_info.pruned_size..],
                        )?;
                    } else {
                        tapes.read_bytes(
                            &db.prunable_blobs[pruning_stripe as usize - 1],
                            tx_info.prunable_blob_idx,
                            &mut blob[(tx_info.pruned_size)..],
                        )?;
                    }

                    Ok(Bytes::from(blob))
                })
                .collect::<Result<_, BlockchainError>>()?,
        )
    };

    let block_blob = {
        let block_blob_end_idx = block_blob_end_idx.map_or_else(
            || {
                let next_block_info =
                    tapes.read_entry(&db.block_infos, (block_height + 1) as u64)?;

                if let Some(info) = next_block_info {
                    return Ok::<_, BlockchainError>(info.pruned_blob_idx);
                }

                Ok(tapes
                    .blob_tape_len(&db.pruned_blobs)
                    .expect("Required tape not found"))
            },
            Ok,
        )?;

        let mut blob = vec![0; u64_to_usize(block_blob_end_idx - block_blob_start_idx)];

        tapes.read_bytes(&db.pruned_blobs, block_blob_start_idx, &mut blob)?;

        Bytes::from(blob)
    };

    Ok(BlockCompleteEntry {
        block: block_blob,
        txs,
        pruned,
        block_weight: if pruned { block_info.weight as u64 } else { 0 },
    })
}

//---------------------------------------------------------------------------------------------------- `get_block_extended_header_*`
/// Retrieve a [`ExtendedBlockHeader`] from the database.
///
/// This extracts all the data from the database tables
/// needed to create a full `ExtendedBlockHeader`.
///
/// # Notes
/// This is slightly more expensive than [`get_block_extended_header_from_height`]
/// (1 more database lookup).
#[doc = doc_error!()]
#[inline]
pub fn get_block_extended_header(
    db: &BlockchainDatabase,
    block_hash: &BlockHash,
    tx_ro: &fjall::Snapshot,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<ExtendedBlockHeader> {
    let block_height = tx_ro
        .get(&db.block_heights, block_hash)?
        .ok_or(BlockchainError::NotFound)?;

    get_block_extended_header_from_height(
        usize::from_le_bytes(block_height.as_ref().try_into().unwrap()),
        tapes,
        db,
    )
}

/// Same as [`get_block_extended_header`] but with a [`BlockHeight`].
#[doc = doc_error!()]
#[expect(
    clippy::missing_panics_doc,
    reason = "The panic is only possible with a corrupt DB"
)]
#[inline]
pub fn get_block_extended_header_from_height(
    block_height: BlockHeight,
    tapes: &tapes::TapesReadTransaction,
    db: &BlockchainDatabase,
) -> DbResult<ExtendedBlockHeader> {
    let block_info = tapes
        .read_entry(&db.block_infos, block_height as u64)?
        .ok_or(BlockchainError::NotFound)?;
    let miner_tx_info = tapes
        .read_entry(&db.tx_infos, block_info.mining_tx_index)?
        .ok_or(BlockchainError::NotFound)?;

    let mut block_header_blob =
        vec![0; u64_to_usize(miner_tx_info.pruned_blob_idx - block_info.pruned_blob_idx)];
    tapes.read_bytes(
        &db.pruned_blobs,
        block_info.pruned_blob_idx,
        &mut block_header_blob,
    )?;

    let block_header = BlockHeader::read(&mut block_header_blob.as_slice()).unwrap();

    let cumulative_difficulty = combine_low_high_bits_to_u128(
        block_info.cumulative_difficulty_low,
        block_info.cumulative_difficulty_high,
    );

    Ok(ExtendedBlockHeader {
        cumulative_difficulty,
        version: HardFork::from_version(block_header.hardfork_version)
            .expect("Stored block must have a valid hard-fork"),
        vote: block_header.hardfork_signal,
        timestamp: block_header.timestamp,
        block_weight: block_info.weight,
        long_term_weight: block_info.long_term_weight,
    })
}

/// Return the top/latest [`ExtendedBlockHeader`] from the database.
#[doc = doc_error!()]
#[inline]
pub fn get_block_extended_header_top(
    db: &BlockchainDatabase,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<(ExtendedBlockHeader, BlockHeight)> {
    let height = u64_to_usize(
        tapes
            .fixed_sized_tape_len(&db.block_infos)
            .expect("Require tape not found"),
    );
    let header = get_block_extended_header_from_height(height, tapes, db)?;
    Ok((header, height))
}

//---------------------------------------------------------------------------------------------------- Block
/// Retrieve a [`Block`] via its [`BlockHeight`].
#[doc = doc_error!()]
#[inline]
pub fn get_block(
    block_height: &BlockHeight,
    blocks_info: Option<&BlockInfo>,
    tapes: &impl tapes::TapesRead,
    db: &BlockchainDatabase,
) -> DbResult<Block> {
    let block_info = match blocks_info {
        Some(blocks_info) => *blocks_info,
        None => tapes
            .read_entry(&db.block_infos, *block_height as u64)?
            .ok_or(BlockchainError::NotFound)?,
    };

    let pruned_end_blob_idx =
        match tapes.read_entry(&db.tx_infos, block_info.mining_tx_index + 1)? {
            Some(tx_info) if tx_info.height == *block_height => tx_info.pruned_blob_idx,
            Some(_) => {
                tapes
                    .read_entry(&db.block_infos, (*block_height + 1) as u64)?
                    .ok_or(BlockchainError::NotFound)?
                    .pruned_blob_idx
            }
            None => tapes
                .blob_tape_len(&db.pruned_blobs)
                .expect("Required tape not found"),
        };

    let mut blob =
        vec![0; usize::try_from(pruned_end_blob_idx - block_info.pruned_blob_idx).unwrap()];

    tapes.read_bytes(&db.pruned_blobs, block_info.pruned_blob_idx, &mut blob)?;

    Ok(Block::read(&mut blob.as_slice())?)
}

/// Retrieve a [`Block`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_by_hash(
    db: &BlockchainDatabase,
    block_hash: &BlockHash,
    tx_ro: &fjall::Snapshot,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<Block> {
    let block_height = tx_ro
        .get(&db.block_heights, block_hash)?
        .ok_or(BlockchainError::NotFound)?;

    get_block(
        &usize::from_le_bytes(block_height.as_ref().try_into().unwrap()),
        None,
        tapes,
        db,
    )
}

//---------------------------------------------------------------------------------------------------- Misc
/// Retrieve a [`BlockHeight`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_height(
    db: &BlockchainDatabase,
    block_hash: &BlockHash,
    tx_ro: &fjall::Snapshot,
) -> DbResult<BlockHeight> {
    let block_height = tx_ro
        .get(&db.block_heights, block_hash)?
        .ok_or(BlockchainError::NotFound)?;

    Ok(usize::from_le_bytes(
        block_height.as_ref().try_into().unwrap(),
    ))
}

/// Check if a block exists in the database.
///
/// # Errors
/// Note that this will never return `Err(RuntimeError::KeyNotFound)`,
/// as in that case, `Ok(false)` will be returned.
///
/// Other errors may still occur.
#[inline]
pub fn block_exists(
    db: &BlockchainDatabase,
    block_hash: &BlockHash,
    tx_ro: &fjall::Snapshot,
) -> DbResult<bool> {
    Ok(tx_ro.contains_key(&db.block_heights, block_hash)?)
}

pub(crate) fn block_height(
    db: &BlockchainDatabase,
    tx_ro: &fjall::Snapshot,
    hash: &[u8; 32],
) -> DbResult<Option<usize>> {
    let Some(block_height) = tx_ro.get(&db.block_heights, hash)? else {
        return Ok(None);
    };

    Ok(Some(usize::from_le_bytes(
        block_height.as_ref().try_into().unwrap(),
    )))
}
