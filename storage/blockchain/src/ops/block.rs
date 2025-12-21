//! Block functions.

use std::cmp::min;
//---------------------------------------------------------------------------------------------------- Import
use std::io::Write;

use bytemuck::TransparentWrapper;
use bytes::Bytes;
use fjall::Readable;
use cuprate_helper::cast::usize_to_u64;
use cuprate_helper::{
    map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits},
    tx::tx_fee,
};
use cuprate_pruning::CRYPTONOTE_PRUNING_LOG_STRIPES;
use cuprate_types::{
    AltBlockInformation, BlockCompleteEntry, ChainId, ExtendedBlockHeader, HardFork,
    PrunedTxBlobEntry, TransactionBlobs, VerifiedBlockInformation, VerifiedTransactionInformation,
};
use heed::types::U64;
use monero_oxide::{
    block::{Block, BlockHeader},
    transaction::Transaction,
};
use tapes::MmapFile;

use crate::database::{
    BLOCK_INFOS, PRUNABLE_BLOBS, PRUNED_BLOBS, RCT_OUTPUTS, TX_INFOS,
    V1_PRUNABLE_BLOBS,
};
use crate::error::{BlockchainError, DbResult};
use crate::Blockchain;
use crate::ops::tx::{add_tx_to_dynamic_tables, add_tx_to_dynamic_tables_fjall, add_tx_to_tapes, remove_tx_from_dynamic_tables};
use crate::types::{Hash32Bytes, RctOutput, TxInfo};
use crate::{
    ops::{alt_block, blockchain::chain_height, macros::doc_error},
    types::{BlockHash, BlockHeight, BlockInfo},
};

//---------------------------------------------------------------------------------------------------- `add_block_*`
pub fn add_blocks_to_tapes(
    blocks: &[VerifiedBlockInformation],
    tapes: &mut tapes::Appender<MmapFile>,
) -> DbResult<()> {
    mod adapters {
        use super::*;

        /// A writer for the pruned tape.
        pub(crate) struct PrunedTapeWriter<'a>(pub &'a [VerifiedBlockInformation]);

        impl tapes::Blob for PrunedTapeWriter<'_> {
            fn len(&self) -> usize {
                self.0
                    .iter()
                    .map(|block| {
                        block.block_blob.len()
                            + block
                                .txs
                                .iter()
                                .map(|tx| tx.tx_pruned.len() + 32)
                                .sum::<usize>()
                    })
                    .sum::<usize>()
            }
            fn write(&self, mut buf: &mut [u8]) {
                for block in self.0 {
                    buf.write_all(block.block_blob.as_slice()).unwrap();

                    for tx in &block.txs {
                        buf.write_all(tx.tx_pruned.as_slice()).unwrap();
                        let prunable_hash =
                            if tx.tx_prunable_blob.is_empty() || tx.tx.version() == 1 {
                                [0; 32]
                            } else {
                                monero_oxide::primitives::keccak256(&tx.tx_prunable_blob)
                            };
                        buf.write_all(&prunable_hash).unwrap();
                    }
                }

                assert!(buf.is_empty());
            }
        }

        /// A writer to write all prunable blobs across all blocks into a tape.
        pub(crate) struct PrunableTapeWriter<'a>(pub &'a [VerifiedBlockInformation]);

        impl tapes::Blob for PrunableTapeWriter<'_> {
            fn len(&self) -> usize {
                self.0
                    .iter()
                    .map(|block| {
                        block
                            .txs
                            .iter()
                            .map(|tx| {
                                if tx.tx.version() != 1 {
                                    tx.tx_prunable_blob.len()
                                } else {
                                    0
                                }
                            })
                            .sum::<usize>()
                    })
                    .sum::<usize>()
            }
            fn write(&self, mut buf: &mut [u8]) {
                for block in self.0 {
                    for tx in &block.txs {
                        if tx.tx.version() != 1 {
                            buf.write_all(&tx.tx_prunable_blob).unwrap();
                        }
                    }
                }
                assert!(buf.is_empty());
            }
        }
        /// A writer to write all prunable blobs across all blocks into a tape.
        pub(crate) struct V1PrunableTapeWriter<'a>(pub &'a [VerifiedBlockInformation]);

        impl tapes::Blob for V1PrunableTapeWriter<'_> {
            fn len(&self) -> usize {
                self.0
                    .iter()
                    .map(|block| {
                        block
                            .txs
                            .iter()
                            .map(|tx| {
                                if tx.tx.version() == 1 {
                                    tx.tx_prunable_blob.len()
                                } else {
                                    0
                                }
                            })
                            .sum::<usize>()
                    })
                    .sum::<usize>()
            }
            fn write(&self, mut buf: &mut [u8]) {
                for block in self.0 {
                    for tx in &block.txs {
                        if tx.tx.version() == 1 {
                            buf.write_all(&tx.tx_prunable_blob).unwrap();
                        }
                    }
                }
                assert!(buf.is_empty());
            }
        }
    }
    use adapters::*;

    // Write all the blocks pruned blobs to the pruned tape
    let w = PrunedTapeWriter(blocks);
    let mut pruned_tape_index = tapes.blob_tape_appender(PRUNED_BLOBS).push_bytes(&w)?;

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

    for blocks in [first_stripe, next_stripe] {
        if blocks.is_empty() {
            continue;
        }

        let stripe =
            cuprate_pruning::get_block_pruning_stripe(blocks[0].height, usize::MAX, 3).unwrap();
        let w = PrunableTapeWriter(&blocks);
        let mut v2_prunable_index = tapes
            .blob_tape_appender(PRUNABLE_BLOBS[stripe as usize - 1])
            .push_bytes(&w)?;

        let w = V1PrunableTapeWriter(&blocks);
        let mut v1_prunable_index = tapes.blob_tape_appender(V1_PRUNABLE_BLOBS).push_bytes(&w)?;

        let mut numb_rct_outs = tapes
            .fixed_sized_tape_appender::<RctOutput>(RCT_OUTPUTS)
            .len() as u64;

        for block in blocks {
            let block_pruned_blob_idx = pruned_tape_index;
            let block_v1_prunable_idx = v1_prunable_index;
            let block_v2_prunable_idx = v2_prunable_index;

            let header_len = block.block.header.serialize().len();

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
                    tapes,
                )?
            };

            pruned_tape_index += block.block_blob.len();

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
                    tapes,
                )?;

                pruned_tape_index += tx.tx_pruned.len() + 32;
                if tx.tx.version() == 1 {
                    v1_prunable_index += tx.tx_prunable_blob.len();
                } else {
                    v2_prunable_index += tx.tx_prunable_blob.len();
                }
            }

            let mut block_info_appender = tapes.fixed_sized_tape_appender::<BlockInfo>(BLOCK_INFOS);
            // `saturating_add` is used here as cumulative generated coins overflows due to tail emission.
            let cumulative_generated_coins = block_info_appender
                .reader_slice()?
                .get(block.height.saturating_sub(1))
                .map(|i| i.cumulative_generated_coins)
                .unwrap_or(0)
                .saturating_add(block.generated_coins);

            let (cumulative_difficulty_low, cumulative_difficulty_high) =
                split_u128_into_low_high_bits(block.cumulative_difficulty);

            block_info_appender.slice_to_write(1)?[0] = BlockInfo {
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
            };
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
// no inline, too big.
pub fn add_block_to_dynamic_tables(
    db: &Blockchain,
    block: &VerifiedBlockInformation,
    numb_transactions: &mut usize,
    tx_rw: &mut heed::RwTxn,
) -> DbResult<()> {
    //------------------------------------------------------ Check preconditions first

    // Cast height to `u32` for storage (handled at top of function).
    // Panic (should never happen) instead of allowing DB corruption.
    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1560020991>
    assert!(
        u32::try_from(block.height).is_ok(),
        "block.height ({}) > u32::MAX",
        block.height,
    );

    let chain_height = chain_height(db, tx_rw)?;

    //------------------------------------------------------ Transaction / Outputs / Key Images
    // Add the miner transaction first.
    let tx = block.block.miner_transaction();
    add_tx_to_dynamic_tables(
        db,
        &tx.clone().into(),
        *numb_transactions,
        &tx.hash(),
        &chain_height,
        tx_rw,
    )?;
    *numb_transactions += 1;

    for tx in &block.txs {
        add_tx_to_dynamic_tables(
            db,
            &tx.tx,
            *numb_transactions,
            &tx.tx_hash,
            &chain_height,
            tx_rw,
        )?;
        *numb_transactions += 1;
    }

    db.block_heights
        .put(tx_rw, &block.block_hash, &block.height)?;

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
// no inline, too big.
pub fn add_block_to_dynamic_tables_fjall(
    db: &Blockchain,
    block: &VerifiedBlockInformation,
    numb_transactions: &mut usize,
    tx_rw: &mut fjall::SingleWriterWriteTx,
) -> DbResult<()> {
    //------------------------------------------------------ Check preconditions first

    // Cast height to `u32` for storage (handled at top of function).
    // Panic (should never happen) instead of allowing DB corruption.
    // <https://github.com/Cuprate/cuprate/pull/102#discussion_r1560020991>
    assert!(
        u32::try_from(block.height).is_ok(),
        "block.height ({}) > u32::MAX",
        block.height,
    );

    let chain_height = tx_rw.last_key_value(&db.block_heights_fjall).map_or(0, |kv| usize::from_le_bytes(kv.value().unwrap().as_ref().try_into().unwrap()));

    //------------------------------------------------------ Transaction / Outputs / Key Images
    // Add the miner transaction first.
    let tx = block.block.miner_transaction();
    add_tx_to_dynamic_tables_fjall(
        db,
        &tx.clone().into(),
        *numb_transactions,
        &tx.hash(),
        &chain_height,
        tx_rw,
    )?;
    *numb_transactions += 1;

    for tx in &block.txs {
        add_tx_to_dynamic_tables_fjall(
            db,
            &tx.tx,
            *numb_transactions,
            &tx.tx_hash,
            &chain_height,
            tx_rw,
        )?;
        *numb_transactions += 1;
    }

    tx_rw.insert(&db.block_heights_fjall, &block.block_hash, &block.height.to_le_bytes());

    Ok(())
}


//---------------------------------------------------------------------------------------------------- `pop_block`
/// Remove the top/latest block from the database.
///
/// The removed block's data is returned.
///
/// If a [`ChainId`] is specified the popped block will be added to the alt block tables under
/// that [`ChainId`]. Otherwise, the block will be completely removed from the DB.
#[doc = doc_error!()]
///
/// In `pop_block()`'s case, [`RuntimeError::KeyNotFound`]
/// will be returned if there are no blocks left.
// no inline, too big
pub fn pop_block(
    db: &Blockchain,
    move_to_alt_chain: Option<ChainId>,
    tx_rw: &mut heed::RwTxn,
    tapes: &mut tapes::Popper<MmapFile>,
) -> DbResult<(BlockHeight, BlockHash, Block)> {
    //------------------------------------------------------ Block Info
    let mut block_info_tape = tapes.fixed_sized_tape_popper::<BlockInfo>(BLOCK_INFOS);

    // Remove block data from tables.
    let (block_height, &block_info) = block_info_tape
        .pop_last()
        .ok_or(BlockchainError::NotFound)?;

    db.block_heights
        .delete(tx_rw, &block_info.block_hash)?;

    drop(block_info_tape);
    // Block blobs.
    //
    // We deserialize the block header blob and mining transaction blob
    // to form a `Block`, such that we can remove the associated transactions
    // later.
    let pruned_blobs = tapes.blob_tape_tape_reader(PRUNED_BLOBS);

    let block = Block::read(&mut pruned_blobs.get(block_info.pruned_blob_idx..).unwrap()).unwrap();

    drop(pruned_blobs);
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

    tapes
        .blob_tape_popper(PRUNED_BLOBS)
        .set_new_len(block_info.pruned_blob_idx);
    tapes
        .blob_tape_popper(V1_PRUNABLE_BLOBS)
        .set_new_len(block_info.v1_prunable_blob_idx);
    let stripe = cuprate_pruning::get_block_pruning_stripe(block_height, usize::MAX, 3).unwrap();
    tapes
        .blob_tape_popper(PRUNABLE_BLOBS[stripe as usize - 1])
        .set_new_len(block_info.prunable_blob_idx);

    tapes
        .fixed_sized_tape_popper::<TxInfo>(TX_INFOS)
        .set_new_len(block_info.mining_tx_index);

    let cumulative_rct_outs = tapes
        .fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS)
        .get(block_height - 1)
        .map_or(0, |info| info.cumulative_rct_outs);

    tapes
        .fixed_sized_tape_popper::<RctOutput>(RCT_OUTPUTS)
        .set_new_len(cumulative_rct_outs as usize);

    Ok((block_height, block_info.block_hash, block))
}

//---------------------------------------------------------------------------------------------------- `get_block_complete_entry_*`
/// Retrieve a [`BlockCompleteEntry`] from the database.
///
#[doc = doc_error!()]
pub fn get_block_complete_entry(
    db: &Blockchain,
    block_hash: &BlockHash,
    pruned: bool,
    tx_ro: &heed::RoTxn,
    tapes: &tapes::Reader<MmapFile>,
) -> DbResult<BlockCompleteEntry> {
    let block_height = db.block_heights
        .get(tx_ro, &block_hash)?
        .ok_or(BlockchainError::NotFound)?;
    get_block_complete_entry_from_height(block_height, pruned, tapes)
}

/// Retrieve a [`BlockCompleteEntry`] from the database.
///
#[doc = doc_error!()]
pub fn get_block_complete_entry_from_height(
    block_height: BlockHeight,
    pruned: bool,
    tapes: &tapes::Reader<MmapFile>,
) -> DbResult<BlockCompleteEntry> {
    let pruning_stripe = cuprate_pruning::get_block_pruning_stripe(
        block_height,
        usize::MAX,
        CRYPTONOTE_PRUNING_LOG_STRIPES,
    )
    .unwrap();

    let pruned_tape_reader = tapes.blob_tape_tape_slice(PRUNED_BLOBS);
    let block_info_tape_reader = tapes.fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS);

    let prunable_tape_reader =
        tapes.blob_tape_tape_slice(PRUNABLE_BLOBS[pruning_stripe as usize - 1]);
    let v1_prunable_tape_reader = tapes.blob_tape_tape_slice(V1_PRUNABLE_BLOBS);

    let tx_infos_reader = tapes.fixed_sized_tape_slice::<TxInfo>(TX_INFOS);

    let block_info = block_info_tape_reader
        .get(block_height)
        .ok_or(BlockchainError::NotFound)?;

    let block_blob_start_idx = block_info.pruned_blob_idx;
    let mut block_blob_end_idx = None;

    let mut txs = Vec::with_capacity(32);

    let mut i = 1;
    while let Some(tx_info) = tx_infos_reader.get((block_info.mining_tx_index + i)) {
        if tx_info.height != block_height {
            break;
        }

        block_blob_end_idx.get_or_insert(tx_info.pruned_blob_idx);

        if pruned {
            let blob = &pruned_tape_reader
                [tx_info.pruned_blob_idx..(tx_info.pruned_blob_idx + tx_info.pruned_size)];
            let prunable_hash = &pruned_tape_reader[tx_info.pruned_blob_idx + tx_info.pruned_size
                ..(32 + tx_info.pruned_blob_idx + tx_info.pruned_size)];

            txs.push((blob, prunable_hash));
        } else {
            let pruned_blob = &pruned_tape_reader
                [tx_info.pruned_blob_idx..(tx_info.pruned_blob_idx + tx_info.pruned_size)];

            let prunable_blob = &if tx_info.rct_output_start_idx == u64::MAX {
                &v1_prunable_tape_reader
            } else {
                &prunable_tape_reader
            }[tx_info.prunable_blob_idx..(tx_info.prunable_blob_idx + tx_info.prunable_size)];

            txs.push((pruned_blob, prunable_blob));
        }

        i += 1;
    }

    let txs = if pruned {
        TransactionBlobs::Pruned(
            txs.into_iter()
                .map(|(pruned, prunable_hash)| PrunedTxBlobEntry {
                    blob: Bytes::copy_from_slice(pruned),
                    prunable_hash: Bytes::copy_from_slice(prunable_hash).try_into().unwrap(),
                })
                .collect(),
        )
    } else {
        TransactionBlobs::Normal(
            txs.into_iter()
                .map(|(pruned, prunable)| {
                    let buf = [pruned, prunable].concat();
                    Bytes::from(buf)
                })
                .collect(),
        )
    };

    let block_blob = {
        let block_blob_end_idx = block_blob_end_idx.unwrap_or_else(|| {
            let next_block_info = block_info_tape_reader.get((block_height + 1));

            if let Some(info) = next_block_info {
                return info.pruned_blob_idx;
            };

            pruned_tape_reader.len()
        });

        let blob = &pruned_tape_reader[block_blob_start_idx..block_blob_end_idx];

        Bytes::copy_from_slice(blob)
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
    db: &Blockchain,
    block_hash: &BlockHash,
    tx_ro: &heed::RoTxn,
    tapes: &tapes::Reader<MmapFile>,
) -> DbResult<ExtendedBlockHeader> {
    get_block_extended_header_from_height(
        db.block_heights
            .get(tx_ro, block_hash)?
            .ok_or(BlockchainError::NotFound)?,
        tapes,
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
    tapes: &tapes::Reader<MmapFile>,
) -> DbResult<ExtendedBlockHeader> {
    let blocks_infos = tapes.fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS);

    let block_info = blocks_infos
        .get(block_height)
        .ok_or(BlockchainError::NotFound)?;
    let miner_tx_info = *tapes
        .fixed_sized_tape_slice::<TxInfo>(TX_INFOS)
        .get(block_info.mining_tx_index)
        .ok_or(BlockchainError::NotFound)?;

    let pruned_tape = tapes.blob_tape_tape_slice(PRUNED_BLOBS);
    let mut block_header_blob = pruned_tape
        .get(block_info.pruned_blob_idx..miner_tx_info.pruned_blob_idx)
        .ok_or(BlockchainError::NotFound)?;

    let block_header = BlockHeader::read(&mut block_header_blob)?;

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
    tapes: &tapes::Reader<MmapFile>,
) -> DbResult<(ExtendedBlockHeader, BlockHeight)> {
    let blocks_infos = tapes.fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS);
    let height = blocks_infos.len();
    let header = get_block_extended_header_from_height(height, tapes)?;
    Ok((header, height))
}

//---------------------------------------------------------------------------------------------------- Block
/// Retrieve a [`Block`] via its [`BlockHeight`].
#[doc = doc_error!()]
#[inline]
pub fn get_block(block_height: &BlockHeight, tapes: &tapes::Reader<MmapFile>) -> DbResult<Block> {
    let block_infos = tapes.fixed_sized_tape_slice::<BlockInfo>(BLOCK_INFOS);
    let block_info = block_infos
        .get(*block_height)
        .ok_or(BlockchainError::NotFound)?;

    let pruned_blobs = tapes.blob_tape_tape_slice(PRUNED_BLOBS);

    Ok(Block::read(
        &mut pruned_blobs
            .get(block_info.pruned_blob_idx..)
            .ok_or(BlockchainError::NotFound)?,
    )?)
}

/// Retrieve a [`Block`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_by_hash(
    db: &Blockchain,
    block_hash: &BlockHash,
    tx_ro: &heed::RoTxn,
    tapes: &tapes::Reader<MmapFile>,
) -> DbResult<Block> {
    let block_height = db.block_heights
        .get(tx_ro, block_hash)?
        .ok_or(BlockchainError::NotFound)?;
    get_block(&block_height, tapes)
}

//---------------------------------------------------------------------------------------------------- Misc
/// Retrieve a [`BlockHeight`] via its [`BlockHash`].
#[doc = doc_error!()]
#[inline]
pub fn get_block_height(db: &Blockchain, block_hash: &BlockHash, tx_ro: &heed::RoTxn) -> DbResult<BlockHeight> {
    Ok(db.block_heights
        .get(tx_ro, block_hash)?
        .ok_or(BlockchainError::NotFound)?)
}

/// Check if a block exists in the database.
///
/// # Errors
/// Note that this will never return `Err(RuntimeError::KeyNotFound)`,
/// as in that case, `Ok(false)` will be returned.
///
/// Other errors may still occur.
#[inline]
pub fn block_exists(db: &Blockchain, block_hash: &BlockHash, tx_ro: &heed::RoTxn) -> DbResult<bool> {
    Ok(db.block_heights
        .get(tx_ro, block_hash)?
        .is_some())
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
#[expect(clippy::too_many_lines)]
mod test {
    use pretty_assertions::assert_eq;

    use cuprate_database::{Env, EnvInner, TxRw};
    use cuprate_test_utils::data::{BLOCK_V16_TX0, BLOCK_V1_TX2, BLOCK_V9_TX3};

    use crate::{
        ops::tx::{get_tx, tx_exists},
        tables::OpenTables,
        tests::{assert_all_tables_are_empty, tmp_concrete_env, AssertTableLen},
    };

    use super::*;

    /// Tests all above block functions.
    ///
    /// Note that this doesn't test the correctness of values added, as the
    /// functions have a pre-condition that the caller handles this.
    ///
    /// It simply tests if the proper tables are mutated, and if the data
    /// stored and retrieved is the same.
    #[test]
    fn all_block_functions() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let mut blocks = [
            BLOCK_V1_TX2.clone(),
            BLOCK_V9_TX3.clone(),
            BLOCK_V16_TX0.clone(),
        ];
        // HACK: `add_block()` asserts blocks with non-sequential heights
        // cannot be added, to get around this, manually edit the block height.
        for (height, block) in blocks.iter_mut().enumerate() {
            block.height = height;
            assert_eq!(block.block.serialize(), block.block_blob);
        }
        let generated_coins_sum = blocks
            .iter()
            .map(|block| block.generated_coins)
            .sum::<u64>();

        // Add blocks.
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            for block in &blocks {
                // println!("add_block: {block:#?}");
                add_block(block, &mut tables).unwrap();
            }

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        // Assert all reads are OK.
        let block_hashes = {
            let tx_ro = env_inner.tx_ro().unwrap();
            let tables = env_inner.open_tables(&tx_ro).unwrap();

            // Assert only the proper tables were added to.
            AssertTableLen {
                block_infos: 3,
                block_header_blobs: 3,
                block_txs_hashes: 3,
                block_heights: 3,
                key_images: 69,
                num_outputs: 41,
                pruned_tx_blobs: 0,
                prunable_hashes: 0,
                outputs: 111,
                prunable_tx_blobs: 0,
                rct_outputs: 8,
                tx_blobs: 8,
                tx_ids: 8,
                tx_heights: 8,
                tx_unlock_time: 3,
            }
            .assert(&tables);

            // Check `cumulative` functions work.
            assert_eq!(
                cumulative_generated_coins(&2, tables.block_infos()).unwrap(),
                generated_coins_sum,
            );

            // Both height and hash should result in getting the same data.
            let mut block_hashes = vec![];
            for block in &blocks {
                println!("blocks.iter(): hash: {}", hex::encode(block.block_hash));

                let height = get_block_height(&block.block_hash, tables.block_heights()).unwrap();

                println!("blocks.iter(): height: {height}");

                assert!(block_exists(&block.block_hash, tables.block_heights()).unwrap());

                let block_header_from_height =
                    get_block_extended_header_from_height(&height, &tables).unwrap();
                let block_header_from_hash =
                    get_block_extended_header(&block.block_hash, &tables).unwrap();

                // Just an alias, these names are long.
                let b1 = block_header_from_hash;
                let b2 = block;
                assert_eq!(b1, block_header_from_height);
                assert_eq!(b1.version.as_u8(), b2.block.header.hardfork_version);
                assert_eq!(b1.vote, b2.block.header.hardfork_signal);
                assert_eq!(b1.timestamp, b2.block.header.timestamp);
                assert_eq!(b1.cumulative_difficulty, b2.cumulative_difficulty);
                assert_eq!(b1.block_weight, b2.weight);
                assert_eq!(b1.long_term_weight, b2.long_term_weight);

                block_hashes.push(block.block_hash);

                // Assert transaction reads are OK.
                for (i, tx) in block.txs.iter().enumerate() {
                    println!("tx_hash: {:?}", hex::encode(tx.tx_hash));

                    assert!(tx_exists(&tx.tx_hash, tables.tx_ids()).unwrap());

                    let tx2 = get_tx(&tx.tx_hash, tables.tx_ids(), tables.tx_blobs()).unwrap();

                    assert_eq!(tx.tx_blob, tx2.serialize());
                    assert_eq!(tx.tx_weight, tx2.weight());
                    assert_eq!(tx.tx_hash, block.block.transactions[i]);
                    assert_eq!(tx.tx_hash, tx2.hash());
                }
            }

            block_hashes
        };

        {
            let len = block_hashes.len();
            let hashes: Vec<String> = block_hashes.iter().map(hex::encode).collect();
            println!("block_hashes: len: {len}, hashes: {hashes:?}");
        }

        // Remove the blocks.
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            for block_hash in block_hashes.into_iter().rev() {
                println!("pop_block(): block_hash: {}", hex::encode(block_hash));

                let (_popped_height, popped_hash, _popped_block) =
                    pop_block(None, &mut tables).unwrap();

                assert_eq!(block_hash, popped_hash);

                assert!(matches!(
                    get_block_extended_header(&block_hash, &tables),
                    Err(RuntimeError::KeyNotFound)
                ));
            }

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        assert_all_tables_are_empty(&env);
    }

    /// We should panic if: `block.height` > `u32::MAX`
    #[test]
    #[should_panic(expected = "block.height (4294967296) > u32::MAX")]
    fn block_height_gt_u32_max() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let tx_rw = env_inner.tx_rw().unwrap();
        let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

        let mut block = BLOCK_V9_TX3.clone();

        block.height = cuprate_helper::cast::u32_to_usize(u32::MAX) + 1;
        add_block(&block, &mut tables).unwrap();
    }

    /// We should panic if: `block.height` != the chain height
    #[test]
    #[should_panic(
        expected = "assertion `left == right` failed: block.height (123) != chain_height (1)\n  left: 123\n right: 1"
    )]
    fn block_height_not_chain_height() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let tx_rw = env_inner.tx_rw().unwrap();
        let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

        let mut block = BLOCK_V9_TX3.clone();
        // HACK: `add_block()` asserts blocks with non-sequential heights
        // cannot be added, to get around this, manually edit the block height.
        block.height = 0;

        // OK, `0 == 0`
        assert_eq!(block.height, 0);
        add_block(&block, &mut tables).unwrap();

        // FAIL, `123 != 1`
        block.height = 123;
        add_block(&block, &mut tables).unwrap();
    }
}
