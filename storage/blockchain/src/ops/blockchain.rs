//! Blockchain functions - chain height, generated coins, etc.

use fjall::Readable;
use std::io;
use tapes::TapesRead;
//---------------------------------------------------------------------------------------------------- Import

use crate::error::{BlockchainError, DbResult};
use crate::BlockchainDatabase;
use crate::{
    ops::{block, macros::doc_error},
    types::{BlockHash, BlockHeight},
};

//---------------------------------------------------------------------------------------------------- Free Functions
/// Retrieve the height of the chain.
///
/// This returns the chain-tip, not the [`top_block_height`].
///
/// For example:
/// - The blockchain has 0 blocks => this returns `0`
/// - The blockchain has 1 block (height 0) => this returns `1`
/// - The blockchain has 2 blocks (height 1) => this returns `2`
///
/// So the height of a new block would be `chain_height()`.
#[doc = doc_error!()]
#[inline]
pub fn chain_height(
    db: &BlockchainDatabase,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<BlockHeight> {
    #[expect(clippy::cast_possible_truncation, reason = "we enforce 64-bit")]
    Ok(tapes
        .fixed_sized_tape_len(&db.block_infos)
        .expect("Required tape must exists") as usize)
}

/// Retrieve the height of the top block.
///
/// This returns the height of the top block, not the [`chain_height`].
///
/// For example:
/// - The blockchain has 0 blocks => this returns `Err(RuntimeError::KeyNotFound)`
/// - The blockchain has 1 block (height 0) => this returns `Ok(0)`
/// - The blockchain has 2 blocks (height 1) => this returns `Ok(1)`
///
/// Note that in cases where no blocks have been written to the
/// database yet, an error is returned: `Err(RuntimeError::KeyNotFound)`.
///
#[doc = doc_error!()]
#[inline]
pub fn top_block_height(
    db: &BlockchainDatabase,
    tapes: &tapes::TapesReadTransaction,
) -> DbResult<BlockHeight> {
    match chain_height(db, tapes)? {
        0 => Err(BlockchainError::NotFound),
        height => Ok(height - 1),
    }
}

/// Find the split point between our chain and a list of [`BlockHash`]s from another chain.
///
/// This function accepts chains in chronological and reverse chronological order, however
/// if the wrong order is specified the return value is meaningless.
///
/// For chronologically ordered chains this will return the index of the first unknown, for reverse
/// chronologically ordered chains this will return the index of the first known.
///
/// If all blocks are known for chronologically ordered chains or unknown for reverse chronologically
/// ordered chains then the length of the `block_ids` will be returned.
#[doc = doc_error!()]
#[inline]
pub fn find_split_point(
    db: &BlockchainDatabase,
    block_ids: &[BlockHash],
    chronological_order: bool,
    include_alt_blocks: bool,
    tx_ro: &fjall::Snapshot,
) -> DbResult<usize> {
    let mut err = None;

    let block_exists = |block_id| tx_ro.contains_key(&db.block_heights, block_id);

    // Do a binary search to find the first unknown/known block in the batch.
    let idx = block_ids.partition_point(|block_id| {
        match block_exists(*block_id) {
            Ok(exists) => exists == chronological_order,
            Err(e) => {
                err.get_or_insert(e);
                // if this happens the search is scrapped, just return `false` back.
                false
            }
        }
    });

    if let Some(e) = err {
        panic!();
    }

    Ok(idx)
}
