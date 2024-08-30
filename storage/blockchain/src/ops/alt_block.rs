use std::cmp::max;

use bytemuck::TransparentWrapper;
use cuprate_database::{DatabaseRo, DatabaseRw, RuntimeError, StorableVec};
use cuprate_helper::map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits};
use cuprate_types::{
    AltBlockInformation, Chain, ChainId, ExtendedBlockHeader, HardFork,
    VerifiedTransactionInformation,
};
use monero_serai::block::BlockHeader;

use crate::{
    ops::block::{get_block_extended_header_from_height, get_block_info},
    tables::{Tables, TablesMut},
    types::{
        AltBlockHeight, AltChainInfo, AltTransactionInfo, BlockHash, BlockHeight,
        CompactAltBlockInfo,
    },
};

pub fn add_alt_block(
    alt_block: &AltBlockInformation,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    let alt_block_height = AltBlockHeight {
        chain_id: alt_block.chain_id.into(),
        height: alt_block.height,
    };

    tables
        .alt_block_heights_mut()
        .put(&alt_block.block_hash, &alt_block_height)?;

    check_add_alt_chain_info(&alt_block_height, &alt_block.block.header.previous, tables)?;

    let (cumulative_difficulty_low, cumulative_difficulty_high) =
        split_u128_into_low_high_bits(alt_block.cumulative_difficulty);

    let alt_block_info = CompactAltBlockInfo {
        block_hash: alt_block.block_hash,
        pow_hash: alt_block.pow_hash,
        height: alt_block.height,
        weight: alt_block.weight,
        long_term_weight: alt_block.long_term_weight,
        cumulative_difficulty_low,
        cumulative_difficulty_high,
    };

    tables
        .alt_blocks_info_mut()
        .put(&alt_block_height, &alt_block_info)?;

    tables.alt_block_blobs_mut().put(
        &alt_block_height,
        StorableVec::wrap_ref(&alt_block.block_blob),
    )?;

    for tx in &alt_block.txs {
        add_alt_transaction(&tx, tables)?;
    }

    Ok(())
}

pub fn add_alt_transaction(
    tx: &VerifiedTransactionInformation,
    tables: &mut impl TablesMut,
) -> Result<(), RuntimeError> {
    if tables.tx_ids().get(&tx.tx_hash).is_ok()
        || tables.alt_transaction_infos().get(&tx.tx_hash).is_ok()
    {
        return Ok(());
    }

    tables.alt_transaction_infos_mut().put(
        &tx.tx_hash,
        &AltTransactionInfo {
            tx_weight: tx.tx_weight,
            fee: tx.fee,
            tx_hash: tx.tx_hash,
        },
    )?;

    tables
        .alt_transaction_blobs_mut()
        .put(&tx.tx_hash, StorableVec::wrap_ref(&tx.tx_blob))
}

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
        },
    )
}

pub fn alt_block_hash(
    block_height: &BlockHeight,
    alt_chain: ChainId,
    tables: &mut impl Tables,
) -> Result<BlockHash, RuntimeError> {
    let alt_chains = tables.alt_chain_infos();

    let original_chain = {
        let mut chain = alt_chain.into();
        loop {
            let chain_info = alt_chains.get(&chain)?;

            if chain_info.common_ancestor_height < *block_height {
                break Chain::Alt(chain.into());
            }

            match chain_info.parent_chain.into() {
                Chain::Main => break Chain::Main,
                Chain::Alt(alt_chain_id) => {
                    chain = alt_chain_id.into();
                    continue;
                }
            }
        }
    };

    match original_chain {
        Chain::Main => {
            get_block_info(&block_height, tables.block_infos()).map(|info| info.block_hash)
        }
        Chain::Alt(chain_id) => tables
            .alt_blocks_info()
            .get(&AltBlockHeight {
                chain_id: chain_id.into(),
                height: *block_height,
            })
            .map(|info| info.block_hash),
    }
}

pub fn alt_extended_headers_in_range(
    range: std::ops::Range<BlockHeight>,
    alt_chain: ChainId,
    tables: &impl Tables,
) -> Result<Vec<ExtendedBlockHeader>, RuntimeError> {
    // TODO: this function does not use rayon, however it probably should.

    let mut ranges = Vec::with_capacity(5);
    let alt_chains = tables.alt_chain_infos();

    let mut i = range.end;
    let mut current_chain_id = alt_chain.into();
    while i > range.start {
        let chain_info = alt_chains.get(&current_chain_id)?;

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

    let res = ranges
        .into_iter()
        .rev()
        .map(|(chain, range)| {
            range.into_iter().map(move |height| match chain {
                Chain::Main => get_block_extended_header_from_height(&height, tables),
                Chain::Alt(chain_id) => get_alt_block_extended_header_from_height(
                    &AltBlockHeight {
                        chain_id: chain_id.into(),
                        height,
                    },
                    tables,
                ),
            })
        })
        .flatten()
        .collect::<Result<_, _>>()?;

    Ok(res)
}

pub fn get_alt_block_extended_header_from_height(
    height: &AltBlockHeight,
    table: &impl Tables,
) -> Result<ExtendedBlockHeader, RuntimeError> {
    let block_info = table.alt_blocks_info().get(height)?;

    let block_blob = table.alt_block_blobs().get(height)?.0;

    let block_header = BlockHeader::read(&mut block_blob.as_slice())?;

    Ok(ExtendedBlockHeader {
        version: HardFork::from_version(0).expect("Block in DB must have correct version"),
        vote: block_header.hardfork_version,
        timestamp: block_header.timestamp,
        cumulative_difficulty: combine_low_high_bits_to_u128(
            block_info.cumulative_difficulty_low,
            block_info.cumulative_difficulty_high,
        ),
        block_weight: block_info.weight,
        long_term_weight: block_info.long_term_weight,
    })
}
