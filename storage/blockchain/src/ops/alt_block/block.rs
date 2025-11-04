use bytemuck::TransparentWrapper;
use monero_oxide::block::{Block, BlockHeader};

use cuprate_database::{DatabaseRo, DatabaseRw, DbResult, RuntimeError, StorableVec};
use cuprate_helper::map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits};
use cuprate_types::{AltBlockInformation, Chain, ChainId, ExtendedBlockHeader, HardFork};

use crate::{
    ops::{
        alt_block::{add_alt_transaction_blob, get_alt_transaction, update_alt_chain_info},
        macros::doc_error,
    },
    tables::{Tables, TablesMut},
    types::{AltBlockHeight, BlockHash, BlockHeight, CompactAltBlockInfo},
};
use crate::database::BLOCK_INFOS;
use crate::types::BlockInfo;

/// Flush all alt-block data from all the alt-block tables.
///
/// This function completely empties the alt block tables.
pub fn flush_alt_blocks<'a, E: cuprate_database::EnvInner<'a>>(
    env_inner: &E,
    tx_rw: &mut E::Rw<'_>,
) -> DbResult<()> {
    use crate::tables::{
        AltBlockBlobs, AltBlockHeights, AltBlocksInfo, AltChainInfos, AltTransactionBlobs,
        AltTransactionInfos,
    };

    env_inner.clear_db::<AltChainInfos>(tx_rw)?;
    env_inner.clear_db::<AltBlockHeights>(tx_rw)?;
    env_inner.clear_db::<AltBlocksInfo>(tx_rw)?;
    env_inner.clear_db::<AltBlockBlobs>(tx_rw)?;
    env_inner.clear_db::<AltTransactionBlobs>(tx_rw)?;
    env_inner.clear_db::<AltTransactionInfos>(tx_rw)
}

/// Add a [`AltBlockInformation`] to the database.
///
/// This extracts all the data from the input block and
/// maps/adds them to the appropriate database tables.
///
#[doc = doc_error!()]
///
/// # Panics
/// This function will panic if:
/// - `alt_block.height` is == `0`
/// - `alt_block.txs.len()` != `alt_block.block.transactions.len()`
///
pub fn add_alt_block(alt_block: &AltBlockInformation, tables: &mut impl TablesMut) -> DbResult<()> {
    let alt_block_height = AltBlockHeight {
        chain_id: alt_block.chain_id.into(),
        height: alt_block.height,
    };

    tables
        .alt_block_heights_mut()
        .put(&alt_block.block_hash, &alt_block_height,         false
        )?;

    update_alt_chain_info(&alt_block_height, &alt_block.block.header.previous, tables)?;

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
        .put(&alt_block_height, &alt_block_info,         false
        )?;

    tables.alt_block_blobs_mut().put(
        &alt_block_height,
        StorableVec::wrap_ref(&alt_block.block_blob),        false

    )?;

    assert_eq!(alt_block.txs.len(), alt_block.block.transactions.len());
    for tx in &alt_block.txs {
        add_alt_transaction_blob(tx, tables)?;
    }

    Ok(())
}

/// Retrieves an [`AltBlockInformation`] from the database.
///
/// This function will look at only the blocks with the given [`AltBlockHeight::chain_id`], no others
/// even if they are technically part of this chain.
#[doc = doc_error!()]
pub fn get_alt_block(
    alt_block_height: &AltBlockHeight,
    tables: &impl Tables,
    tapes: &cuprate_linear_tape::Reader,
) -> DbResult<AltBlockInformation> {
    let block_info = tables.alt_blocks_info().get(alt_block_height)?;

    let block_blob = tables.alt_block_blobs().get(alt_block_height)?.0;

    let block = Block::read(&mut block_blob.as_slice())?;

    let txs = block
        .transactions
        .iter()
        .map(|tx_hash| get_alt_transaction(tx_hash, tables, tapes))
        .collect::<DbResult<_>>()?;

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

/// Retrieves the hash of the block at the given `block_height` on the alt chain with
/// the given [`ChainId`].
///
/// This function will get blocks from the whole chain, for example if you were to ask for height
/// `0` with any [`ChainId`] (as long that chain actually exists) you will get the main chain genesis.
///
#[doc = doc_error!()]
pub fn get_alt_block_hash(
    block_height: &BlockHeight,
    alt_chain: ChainId,
    tables: &impl Tables,
    block_info_tape: &cuprate_linear_tape::LinearTapeReader<BlockInfo>,
) -> DbResult<BlockHash> {
    let alt_chains = tables.alt_chain_infos();

    // First find what [`ChainId`] this block would be stored under.
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

    // Get the block hash.
    match original_chain {
        Chain::Main => {
            block_info_tape.try_get(*block_height).map(|info| info.block_hash).ok_or(RuntimeError::KeyNotFound)
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

/// Retrieves the [`ExtendedBlockHeader`] of the alt-block with an exact [`AltBlockHeight`].
///
/// This function will look at only the blocks with the given [`AltBlockHeight::chain_id`], no others
/// even if they are technically part of this chain.
///
#[doc = doc_error!()]
pub fn get_alt_block_extended_header_from_height(
    height: &AltBlockHeight,
    table: &impl Tables,
) -> DbResult<ExtendedBlockHeader> {
    let block_info = table.alt_blocks_info().get(height)?;

    let block_blob = table.alt_block_blobs().get(height)?.0;

    let block_header = BlockHeader::read(&mut block_blob.as_slice())?;

    Ok(ExtendedBlockHeader {
        version: HardFork::from_version(block_header.hardfork_version)
            .expect("Block in DB must have correct version"),
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
    use cuprate_test_utils::data::{BLOCK_V16_TX0, BLOCK_V1_TX2, BLOCK_V9_TX3};
    use cuprate_types::{Chain, ChainId};

    use crate::{
        ops::{
            alt_block::{
                add_alt_block, flush_alt_blocks, get_alt_block,
                get_alt_block_extended_header_from_height, get_alt_block_hash,
                get_alt_chain_history_ranges,
            },
            block::{add_block, pop_block},
        },
        tables::{OpenTables, Tables},
        tests::{assert_all_tables_are_empty, map_verified_block_to_alt, tmp_concrete_env},
        types::AltBlockHeight,
    };

    #[test]
    fn all_alt_blocks() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        assert_all_tables_are_empty(&env);

        let chain_id = ChainId(NonZero::new(1).unwrap());

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

                let headers = get_alt_chain_history_ranges(
                    0..(height + 1),
                    chain_id,
                    tables.alt_chain_infos(),
                )
                .unwrap();

                assert_eq!(headers.len(), 2);
                assert_eq!(headers[1], (Chain::Main, 0..1));
                assert_eq!(headers[0], (Chain::Alt(chain_id), 1..(height + 1)));

                prev_hash = alt_block.block_hash;

                let header =
                    get_alt_block_extended_header_from_height(&alt_height, &tables).unwrap();

                assert_eq!(header.timestamp, alt_block.block.header.timestamp);
                assert_eq!(header.block_weight, alt_block.weight);
                assert_eq!(header.long_term_weight, alt_block.long_term_weight);
                assert_eq!(
                    header.cumulative_difficulty,
                    alt_block.cumulative_difficulty
                );
                assert_eq!(
                    header.version.as_u8(),
                    alt_block.block.header.hardfork_version
                );
                assert_eq!(header.vote, alt_block.block.header.hardfork_signal);

                let block_hash = get_alt_block_hash(&height, chain_id, &tables).unwrap();

                assert_eq!(block_hash, alt_block.block_hash);
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
