use std::cmp::max;

use cuprate_database::{DatabaseRo, DatabaseRw, RuntimeError};
use cuprate_types::{Chain, ChainId};

use crate::{
    ops::macros::{doc_add_alt_block_inner_invariant, doc_error},
    tables::{AltChainInfos, TablesMut},
    types::{AltBlockHeight, AltChainInfo, BlockHash, BlockHeight},
};

/// Updates the [`AltChainInfo`] with information on a new alt-block.
///
#[doc = doc_add_alt_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Panics
///
/// This will panic if [`AltBlockHeight::height`] == `0`.
pub fn update_alt_chain_info(
    alt_block_height: &AltBlockHeight,
    prev_hash: &BlockHash,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    // try update the info if one exists for this chain.
    let update = tables
        .alt_chain_infos_mut()
        .update(&alt_block_height.chain_id, |mut info| {
            info.chain_height = alt_block_height.height + 1;
            Some(info)
        });

    match update {
        Ok(()) => return Ok(()),
        Err(RuntimeError::KeyNotFound) => (),
        Err(e) => return Err(e),
    }

    // If one doesn't already exist add it.

    let parent_chain = match tables.alt_block_heights().get(prev_hash) {
        Ok(alt_parent_height) => Chain::Alt(alt_parent_height.chain_id.into()),
        Err(RuntimeError::KeyNotFound) => Chain::Main,
        Err(e) => return Err(e),
    };

    tables.alt_chain_infos_mut().put(
        &alt_block_height.chain_id,
        &AltChainInfo {
            parent_chain: parent_chain.into(),
            common_ancestor_height: alt_block_height.height.checked_sub(1).unwrap(),
            chain_height: alt_block_height.height + 1,
        },
    )
}

/// Get the height history of an alt-chain in reverse chronological order.
///
/// Height history is a list of height ranges with the corresponding [`Chain`] they are stored under.
/// For example if your range goes from height `0` the last entry in the list will be [`Chain::Main`]
/// upto the height where the first split occurs.
#[doc = doc_error!()]
pub fn get_alt_chain_history_ranges(
    range: std::ops::Range<BlockHeight>,
    alt_chain: ChainId,
    alt_chain_infos: &impl DatabaseRo<AltChainInfos>,
) -> Result<Vec<(Chain, std::ops::Range<BlockHeight>)>, RuntimeError> {
    let mut ranges = Vec::with_capacity(5);

    let mut i = range.end;
    let mut current_chain_id = alt_chain.into();
    while i > range.start {
        let chain_info = alt_chain_infos.get(&current_chain_id)?;

        let start_height = max(range.start, chain_info.common_ancestor_height + 1);

        ranges.push((Chain::Alt(current_chain_id.into()), start_height..i));
        i = chain_info.common_ancestor_height + 1;

        match chain_info.parent_chain.into() {
            Chain::Main => {
                ranges.push((Chain::Main, range.start..i));
                break;
            }
            Chain::Alt(alt_chain_id) => {
                current_chain_id = alt_chain_id.into();
                continue;
            }
        }
    }

    Ok(ranges)
}