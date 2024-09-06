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
        version: HardFork::from_version(block_header.hardfork_version).expect("Block in DB must have correct version"),
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

#[cfg(test)]
mod tests {
    use std::num::NonZero;
    use cuprate_database::{Env, EnvInner, TxRw};
    use cuprate_test_utils::data::{BLOCK_V1_TX2, BLOCK_V9_TX3, BLOCK_V16_TX0};
    use cuprate_types::ChainId;
    use crate::ops::alt_block::{add_alt_block, flush_alt_blocks, get_alt_block, get_alt_extended_headers_in_range};
    use crate::ops::block::{add_block, pop_block};
    use crate::tables::OpenTables;
    use crate::tests::{assert_all_tables_are_empty, map_verified_block_to_alt, tmp_concrete_env};
    use crate::types::AltBlockHeight;

    #[test]
    fn all_alt_blocks() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let chain_id = ChainId(NonZero::new(1).unwrap()).into();

        // Add initial block.
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            let mut initial_block = BLOCK_V1_TX2.clone();
            initial_block.height = 0;

            add_block(&initial_block, &mut tables).unwrap();

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        let alt_blocks = [
            map_verified_block_to_alt(BLOCK_V9_TX3.clone(), chain_id),
            map_verified_block_to_alt(BLOCK_V16_TX0.clone(), chain_id),
        ];

        // Add alt-blocks
        {
            let tx_rw = env_inner.tx_rw().unwrap();
            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();

            let mut prev_hash = BLOCK_V1_TX2.block_hash;
            for (i, mut alt_block) in alt_blocks.into_iter().enumerate() {
                let height = i + 1;

                alt_block.height = height;
                alt_block.block.header.previous = prev_hash;
                alt_block.block_blob = alt_block.block.serialize();

                add_alt_block(&alt_block, &mut tables).unwrap();

                let alt_height = AltBlockHeight {
                    chain_id: chain_id.into(),
                    height,
                };

                let alt_block_2 = get_alt_block(&alt_height, &tables).unwrap();
                assert_eq!(alt_block.block, alt_block_2.block);

                let headers = get_alt_extended_headers_in_range(0..(height + 1), chain_id, &tables).unwrap();
                assert_eq!(headers.len(), height);

                let last_header = headers.last().unwrap();
                assert_eq!(last_header.timestamp, alt_block.block.header.timestamp);
                assert_eq!(last_header.block_weight, alt_block.weight);
                assert_eq!(last_header.long_term_weight, alt_block.long_term_weight);
                assert_eq!(last_header.cumulative_difficulty, alt_block.cumulative_difficulty);
                assert_eq!(last_header.version.as_u8(), alt_block.block.header.hardfork_version);
                assert_eq!(last_header.vote, alt_block.block.header.hardfork_signal);

                prev_hash = alt_block.block_hash;
            }

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }


        {
            let mut tx_rw = env_inner.tx_rw().unwrap();

            flush_alt_blocks(&env_inner, &mut tx_rw).unwrap();

            let mut tables = env_inner.open_tables_mut(&tx_rw).unwrap();
            pop_block(None, &mut tables).unwrap();

            drop(tables);
            TxRw::commit(tx_rw).unwrap();
        }

        assert_all_tables_are_empty(&env);
    }

}
