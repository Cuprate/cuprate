use crate::error::{BlockchainError, DbResult};
use crate::ops::macros::{doc_add_alt_block_inner_invariant, doc_error};
use crate::types::{AltBlockHeight, AltChainInfo, BlockHash, BlockHeight, RawChainId};
use crate::BlockchainDatabase;
use cuprate_types::{Chain, ChainId};
use fjall::Readable;
use std::cmp::{max, min};

/// Updates the [`AltChainInfo`] with information on a new alt-block.
///
#[doc = doc_add_alt_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Panics
///
/// This will panic if [`AltBlockHeight::height`] == `0`.
pub fn update_alt_chain_info(
    db: &BlockchainDatabase,
    alt_block_height: &AltBlockHeight,
    prev_hash: &BlockHash,
    tx_rw: &mut fjall::OwnedWriteBatch,
) -> DbResult<()> {
    let parent_chain = match db.alt_block_heights.get(prev_hash) {
        Ok(Some(alt_parent_height)) => {
            let alt_parent_height: AltBlockHeight =
                bytemuck::pod_read_unaligned(alt_parent_height.as_ref());
            Chain::Alt(alt_parent_height.chain_id.into())
        }
        Ok(None) => Chain::Main,
        Err(e) => Err(e)?,
    };

    let Some(info) = db
        .alt_chain_infos
        .get(alt_block_height.chain_id.0.to_le_bytes())?
    else {
        tx_rw.insert(
            &db.alt_chain_infos,
            &alt_block_height.chain_id.0.to_le_bytes(),
            bytemuck::bytes_of(&AltChainInfo {
                parent_chain: parent_chain.into(),
                common_ancestor_height: alt_block_height.height.checked_sub(1).unwrap(),
                chain_height: alt_block_height.height + 1,
            }),
        );

        return Ok(());
    };

    let mut info: AltChainInfo = bytemuck::pod_read_unaligned(info.as_ref());

    if info.chain_height < alt_block_height.height + 1 {
        // If the chain height is increasing we only need to update the chain height.
        info.chain_height = alt_block_height.height + 1;
    } else {
        // If the chain height is not increasing we are popping blocks and need to update the
        // split point.
        info.common_ancestor_height = alt_block_height.height.checked_sub(1).unwrap();
        info.parent_chain = parent_chain.into();
    }

    tx_rw.insert(
        &db.alt_chain_infos,
        &alt_block_height.chain_id.0.to_le_bytes(),
        bytemuck::bytes_of(&info),
    );

    Ok(())
}

/// Get the height history of an alt-chain in reverse chronological order.
///
/// Height history is a list of height ranges with the corresponding [`Chain`] they are stored under.
/// For example if your range goes from height `0` the last entry in the list will be [`Chain::Main`]
/// upto the height where the first split occurs.
#[doc = doc_error!()]
pub fn get_alt_chain_history_ranges(
    db: &BlockchainDatabase,
    range: std::ops::Range<BlockHeight>,
    alt_chain: ChainId,
    tx_ro: &fjall::Snapshot,
) -> DbResult<Vec<(Chain, std::ops::Range<BlockHeight>)>> {
    let mut ranges = Vec::with_capacity(5);

    let mut i = range.end;
    let mut current_chain_id: RawChainId = alt_chain.into();
    while i > range.start {
        let chain_info = tx_ro
            .get(&db.alt_chain_infos, current_chain_id.0.to_le_bytes())?
            .ok_or(BlockchainError::NotFound)?;

        let chain_info: AltChainInfo = bytemuck::pod_read_unaligned(chain_info.as_ref());

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
