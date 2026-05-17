use fjall::Readable;
use monero_oxide::block::{Block, BlockHeader};

use cuprate_helper::{
    cast::usize_to_u64,
    map::{combine_low_high_bits_to_u128, split_u128_into_low_high_bits},
};
use cuprate_types::{AltBlockInformation, Chain, ChainId, ExtendedBlockHeader, HardFork};

use crate::{
    error::{BlockchainError, DbResult},
    ops::alt_block::{add_alt_transaction_blob, get_alt_transaction, update_alt_chain_info},
    types::{
        AltBlockHeight, AltChainInfo, BlockHash, BlockHeight, CompactAltBlockInfo, RawChainId,
    },
    BlockchainDatabase,
};

/// Flush all alt-block data from all the alt-block tables.
///
/// This function completely empties the alt block tables.
///
/// **THIS IS NOT ATOMIC**
pub fn flush_alt_blocks(db: &BlockchainDatabase) -> DbResult<()> {
    db.alt_chain_infos.clear()?;
    db.alt_block_heights.clear()?;
    db.alt_block_infos.clear()?;
    db.alt_block_blobs.clear()?;
    db.alt_transaction_blobs.clear()?;
    db.alt_transaction_infos.clear()?;

    Ok(())
}

/// Add a [`AltBlockInformation`] to the database.
///
/// This extracts all the data from the input block and
/// maps/adds them to the appropriate database tables.
///
/// # Panics
/// This function may panic if the given block data is invalid.
///
/// **THIS IS NOT ATOMIC**
pub fn add_alt_block(
    db: &BlockchainDatabase,
    alt_block: &AltBlockInformation,
    tx_rw: &mut fjall::OwnedWriteBatch,
) -> DbResult<()> {
    let alt_block_height = AltBlockHeight {
        chain_id: alt_block.chain_id.into(),
        height: alt_block.height,
    };

    tx_rw.insert(
        &db.alt_block_heights,
        alt_block.block_hash,
        bytemuck::bytes_of(&alt_block_height),
    );
    update_alt_chain_info(db, &alt_block_height, &alt_block.block.header.previous)?;

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

    tx_rw.insert(
        &db.alt_block_infos,
        bytemuck::bytes_of(&alt_block_height),
        bytemuck::bytes_of(&alt_block_info),
    );
    tx_rw.insert(
        &db.alt_block_blobs,
        bytemuck::bytes_of(&alt_block_height),
        &alt_block.block_blob,
    );

    assert_eq!(alt_block.txs.len(), alt_block.block.transactions.len());
    for tx in &alt_block.txs {
        add_alt_transaction_blob(db, tx, tx_rw)?;
    }

    Ok(())
}

/// Retrieves an [`AltBlockInformation`] from the database.
///
/// This function will look at only the blocks with the given [`AltBlockHeight::chain_id`], no others
/// even if they are technically part of this chain.
pub fn get_alt_block_information(
    db: &BlockchainDatabase,
    alt_block_height: &AltBlockHeight,
    tx_ro: &fjall::Snapshot,
) -> DbResult<AltBlockInformation> {
    let block_info = tx_ro
        .get(&db.alt_block_infos, bytemuck::bytes_of(alt_block_height))?
        .ok_or(BlockchainError::NotFound)?;

    let block_info: CompactAltBlockInfo = bytemuck::pod_read_unaligned(block_info.as_ref());

    let block_blob = tx_ro
        .get(&db.alt_block_blobs, bytemuck::bytes_of(alt_block_height))?
        .ok_or(BlockchainError::NotFound)?;

    let block = Block::read(&mut block_blob.as_ref()).unwrap();

    let txs = block
        .transactions
        .iter()
        .map(|tx_hash| get_alt_transaction(db, tx_hash, tx_ro))
        .collect::<DbResult<Vec<_>>>()?;

    Ok(AltBlockInformation {
        block,
        block_blob: block_blob.to_vec(),
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

/// Retrieve an alt [`Block`] via its [`AltBlockHeight`].
///
/// This function will look at only the blocks with the given [`AltBlockHeight::chain_id`], no others
/// even if they are technically part of this chain.
pub fn get_alt_block(
    db: &BlockchainDatabase,
    alt_block_height: &AltBlockHeight,
    tx_ro: &fjall::Snapshot,
) -> DbResult<Block> {
    let block_blob = tx_ro
        .get(&db.alt_block_blobs, bytemuck::bytes_of(alt_block_height))?
        .ok_or(BlockchainError::NotFound)?;

    Ok(Block::read(&mut block_blob.as_ref()).unwrap())
}

/// Retrieves the hash of the block at the given `block_height` on the alt chain with
/// the given [`ChainId`].
///
/// This function will get blocks from the whole chain, for example if you were to ask for height
/// `0` with any [`ChainId`] (as long that chain actually exists) you will get the main chain genesis.
///
pub fn get_alt_block_hash(
    db: &BlockchainDatabase,
    block_height: &BlockHeight,
    alt_chain: ChainId,
    tx_ro: &fjall::Snapshot,
    tapes: &impl tapes::TapesRead,
) -> DbResult<BlockHash> {
    // First find what [`ChainId`] this block would be stored under.
    let original_chain = {
        let mut chain: RawChainId = alt_chain.into();
        loop {
            let chain_info = tx_ro
                .get(&db.alt_chain_infos, chain.0.to_le_bytes())?
                .ok_or(BlockchainError::NotFound)?;

            let chain_info: AltChainInfo = bytemuck::pod_read_unaligned(chain_info.as_ref());

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
        Chain::Main => tapes
            .read_entry(&db.block_infos, usize_to_u64(*block_height))?
            .map(|info| info.block_hash)
            .ok_or(BlockchainError::NotFound),
        Chain::Alt(chain_id) => tx_ro
            .get(
                &db.alt_block_infos,
                bytemuck::bytes_of(&AltBlockHeight {
                    chain_id: chain_id.into(),
                    height: *block_height,
                }),
            )?
            .map(|info| {
                let info: CompactAltBlockInfo = bytemuck::pod_read_unaligned(info.as_ref());
                info.block_hash
            })
            .ok_or(BlockchainError::NotFound),
    }
}

/// Retrieves the [`ExtendedBlockHeader`] of the alt-block with an exact [`AltBlockHeight`].
///
/// This function will look at only the blocks with the given [`AltBlockHeight::chain_id`], no others
/// even if they are technically part of this chain.
pub fn get_alt_block_extended_header_from_height(
    db: &BlockchainDatabase,
    height: &AltBlockHeight,
    tx_ro: &fjall::Snapshot,
) -> DbResult<ExtendedBlockHeader> {
    let block_info = tx_ro
        .get(&db.alt_block_infos, bytemuck::bytes_of(height))?
        .ok_or(BlockchainError::NotFound)?;

    let block_info: CompactAltBlockInfo = bytemuck::pod_read_unaligned(block_info.as_ref());

    let block_blob = tx_ro
        .get(&db.alt_block_blobs, bytemuck::bytes_of(height))?
        .ok_or(BlockchainError::NotFound)?;

    let block_header = BlockHeader::read(&mut block_blob.as_ref())?;

    Ok(ExtendedBlockHeader {
        version: HardFork::from_version(block_header.hardfork_version)
            .expect("Block in DB must have correct version"),
        vote: block_header.hardfork_signal,
        timestamp: block_header.timestamp,
        cumulative_difficulty: combine_low_high_bits_to_u128(
            block_info.cumulative_difficulty_low,
            block_info.cumulative_difficulty_high,
        ),
        block_weight: block_info.weight,
        long_term_weight: block_info.long_term_weight,
    })
}

/// Returns the [`AltBlockHeight`] of a block from its hash, only if it is in an alt chain.
pub(crate) fn alt_block_height(
    db: &BlockchainDatabase,
    tx_ro: &fjall::Snapshot,
    hash: &BlockHash,
) -> DbResult<Option<AltBlockHeight>> {
    let Some(bytes) = tx_ro.get(&db.alt_block_heights, hash)? else {
        return Ok(None);
    };

    Ok(Some(bytemuck::pod_read_unaligned(bytes.as_ref())))
}
