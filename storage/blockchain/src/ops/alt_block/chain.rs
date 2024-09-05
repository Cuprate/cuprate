use crate::tables::{AltChainInfos, TablesMut};
use crate::types::{AltBlockHeight, AltChainInfo, BlockHash, BlockHeight};
use cuprate_database::{DatabaseRo, DatabaseRw, RuntimeError};
use cuprate_types::{Chain, ChainId};
use std::cmp::max;

pub fn check_add_alt_chain_info(
    alt_block_height: &AltBlockHeight,
    prev_hash: &BlockHash,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    match tables.alt_chain_infos().get(&alt_block_height.chain_id) {
        Ok(_) => return Ok(()),
        Err(RuntimeError::KeyNotFound) => (),
        Err(e) => return Err(e),
    }

    let parent_chain = match tables.alt_block_heights().get(prev_hash) {
        Ok(alt_parent_height) => Chain::Alt(alt_parent_height.chain_id.into()),
        Err(RuntimeError::KeyNotFound) => Chain::Main,
        Err(e) => return Err(e),
    };

    tables.alt_chain_infos_mut().put(
        &alt_block_height.chain_id,
        &AltChainInfo {
            parent_chain: parent_chain.into(),
            common_ancestor_height: alt_block_height.height - 1,
            chain_height: alt_block_height.height,
        },
    )
}

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

        ranges.push((chain_info.parent_chain.into(), start_height..i));
        i = chain_info.common_ancestor_height;

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
