use crate::ops::alt_block::{
    add_alt_transaction_blob, check_add_alt_chain_info, get_alt_chain_history_ranges,
    get_alt_transaction,
};
use crate::ops::block::{get_block_extended_header_from_height, get_block_info};
use crate::tables::{Tables, TablesMut};
use crate::types::{AltBlockHeight, BlockHash, BlockHeight, CompactAltBlockInfo};
use bytemuck::TransparentWrapper;
use cuprate_database::{DatabaseRo, DatabaseRw, RuntimeError, StorableVec};
use cuprate_helper::map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits};
use cuprate_types::{AltBlockInformation, Chain, ChainId, ExtendedBlockHeader, HardFork};
use monero_serai::block::{Block, BlockHeader};

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

    assert_eq!(alt_block.txs.len(), alt_block.block.transactions.len());
    for tx in alt_block.txs.iter() {
        add_alt_transaction_blob(tx, tables)?;
    }

    Ok(())
}

pub fn get_alt_block(
    alt_block_height: &AltBlockHeight,
    tables: &impl Tables,
) -> Result<AltBlockInformation, RuntimeError> {
    let block_info = tables.alt_blocks_info().get(alt_block_height)?;

    let block_blob = tables.alt_block_blobs().get(alt_block_height)?.0;

    let block = Block::read(&mut block_blob.as_slice())?;

    let txs = block
        .transactions
        .iter()
        .map(|tx_hash| get_alt_transaction(tx_hash, tables))
        .collect::<Result<_, RuntimeError>>()?;

    Ok(AltBlockInformation {
        block,
        block_blob,
        txs,
        block_hash: block_info.block_hash,
        pow_hash: block_info.pow_hash,
        height: block_info.height,
        weight: block_info.weight,
        long_term_weight: block_info.long_term_weight,
        cumulative_difficulty: combine_low_high_bits_to_u128(
            block_info.cumulative_difficulty_low,
            block_info.cumulative_difficulty_high,
        ),
        chain_id: alt_block_height.chain_id.into(),
    })
}

pub fn get_alt_block_hash(
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

pub fn get_alt_extended_headers_in_range(
    range: std::ops::Range<BlockHeight>,
    alt_chain: ChainId,
    tables: &impl Tables,
) -> Result<Vec<ExtendedBlockHeader>, RuntimeError> {
    // TODO: this function does not use rayon, however it probably should.

    let alt_chains = tables.alt_chain_infos();
    let ranges = get_alt_chain_history_ranges(range, alt_chain, alt_chains)?;

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
