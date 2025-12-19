use std::cmp::{max, min};

use cuprate_types::{Chain, ChainId};

use crate::types::{
    AltBlockHeight, AltChainInfo, BlockHash, BlockHeight, Hash32Bytes, RawChainId, StorableHeed,
};
use crate::Blockchain;
use crate::error::{BlockchainError, DbResult};
use crate::ops::macros::{doc_add_alt_block_inner_invariant, doc_error};

/// Updates the [`AltChainInfo`] with information on a new alt-block.
///
#[doc = doc_add_alt_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Panics
///
/// This will panic if [`AltBlockHeight::height`] == `0`.
pub fn update_alt_chain_info(
    db: &Blockchain,
    alt_block_height: &AltBlockHeight,
    prev_hash: &BlockHash,
    tx_rw: &mut heed::RwTxn,
) -> DbResult<()> {
    let parent_chain = match db.alt_block_heights.get(tx_rw, prev_hash) {
        Ok(Some(alt_parent_height)) => Chain::Alt(alt_parent_height.chain_id.into()),
        Ok(None) => Chain::Main,
        Err(e) => Err(e)?,
    };

    let Some(mut info) = db.alt_chain_infos
        .get(tx_rw, &alt_block_height.chain_id)?
    else {
        db.alt_chain_infos.put(
            tx_rw,
            &alt_block_height.chain_id,
            &AltChainInfo {
                parent_chain: parent_chain.into(),
                common_ancestor_height: alt_block_height.height.checked_sub(1).unwrap(),
                chain_height: alt_block_height.height + 1,
            },
        )?;

        return Ok(());
    };

    if info.chain_height < alt_block_height.height + 1 {
        // If the chain height is increasing we only need to update the chain height.
        info.chain_height = alt_block_height.height + 1;
    } else {
        // If the chain height is not increasing we are popping blocks and need to update the
        // split point.
        info.common_ancestor_height = alt_block_height.height.checked_sub(1).unwrap();
        info.parent_chain = parent_chain.into();
    }

    db.alt_chain_infos
        .put(tx_rw, &alt_block_height.chain_id, &info)?;

    db.alt_chain_infos.put(tx_rw, &alt_block_height.chain_id, &info)?;

    Ok(())
}

/// Get the height history of an alt-chain in reverse chronological order.
///
/// Height history is a list of height ranges with the corresponding [`Chain`] they are stored under.
/// For example if your range goes from height `0` the last entry in the list will be [`Chain::Main`]
/// upto the height where the first split occurs.
#[doc = doc_error!()]
pub fn get_alt_chain_history_ranges(
    db: &Blockchain,
    range: std::ops::Range<BlockHeight>,
    alt_chain: ChainId,
    tx_ro: &heed::RoTxn,
) -> DbResult<Vec<(Chain, std::ops::Range<BlockHeight>)>> {
    let mut ranges = Vec::with_capacity(5);

    let mut i = range.end;
    let mut current_chain_id = alt_chain.into();
    while i > range.start {
        let chain_info = db.alt_chain_infos
            .get(tx_ro, &current_chain_id)?
            .ok_or(BlockchainError::NotFound)?;

        let start_height = max(range.start, chain_info.common_ancestor_height + 1);
        let end_height = min(i, chain_info.chain_height);

        ranges.push((
            Chain::Alt(current_chain_id.into()),
            start_height..end_height,
        ));
        i = chain_info.common_ancestor_height + 1;

        match chain_info.parent_chain.into() {
            Chain::Main => {
                ranges.push((Chain::Main, range.start..i));
                break;
            }
            Chain::Alt(alt_chain_id) => {
                let alt_chain_id = alt_chain_id.into();

                // This shouldn't be possible to hit, however in a test with custom (invalid) block data
                // this caused an infinite loop.
                if alt_chain_id == current_chain_id {
                    return Err(BlockchainError::IO(std::io::Error::other(
                        "Loop detected in ChainIDs, invalid alt chain.",
                    )));
                }

                current_chain_id = alt_chain_id;
                continue;
            }
        }
    }

    Ok(ranges)
}
