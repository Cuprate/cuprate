use bytemuck::TransparentWrapper;

use cuprate_database::{DatabaseRw, RuntimeError, StorableVec, DatabaseRo};
use cuprate_helper::map::split_u128_into_low_high_bits;
use cuprate_types::{AltBlockInformation, Chain, VerifiedTransactionInformation};

use crate::{
    tables::TablesMut,
    types::{AltBlockHeight, AltChainInfo, AltTransactionInfo, BlockHash, CompactAltBlockInfo},
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
