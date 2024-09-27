//! Functions for [`BlockchainReadRequest`] and [`BlockchainWriteRequest`].

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use anyhow::Error;
use cuprate_blockchain::service::BlockchainReadHandle;
use tower::{Service, ServiceExt};

use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain, ExtendedBlockHeader, OutputOnChain,
};

/// [`BlockchainReadRequest::BlockExtendedHeader`].
pub(super) async fn block_extended_header(
    mut blockchain_read: BlockchainReadHandle,
    height: u64,
) -> Result<ExtendedBlockHeader, Error> {
    let BlockchainResponse::BlockExtendedHeader(header) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockExtendedHeader(u64_to_usize(
            height,
        )))
        .await?
    else {
        unreachable!();
    };

    Ok(header)
}

/// [`BlockchainReadRequest::BlockHash`].
pub(super) async fn block_hash(
    mut blockchain_read: BlockchainReadHandle,
    height: u64,
    chain: Chain,
) -> Result<[u8; 32], Error> {
    let BlockchainResponse::BlockHash(hash) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockHash(
            u64_to_usize(height),
            chain,
        ))
        .await?
    else {
        unreachable!();
    };

    Ok(hash)
}

/// [`BlockchainReadRequest::FindBlock`].
pub(super) async fn find_block(
    mut blockchain_read: BlockchainReadHandle,
    block_hash: [u8; 32],
) -> Result<Option<(Chain, usize)>, Error> {
    let BlockchainResponse::FindBlock(option) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::FindBlock(block_hash))
        .await?
    else {
        unreachable!();
    };

    Ok(option)
}

/// [`BlockchainReadRequest::FilterUnknownHashes`].
pub(super) async fn filter_unknown_hashes(
    mut blockchain_read: BlockchainReadHandle,
    block_hashes: HashSet<[u8; 32]>,
) -> Result<HashSet<[u8; 32]>, Error> {
    let BlockchainResponse::FilterUnknownHashes(output) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::FilterUnknownHashes(block_hashes))
        .await?
    else {
        unreachable!();
    };

    Ok(output)
}

/// [`BlockchainReadRequest::BlockExtendedHeaderInRange`]
pub(super) async fn block_extended_header_in_range(
    mut blockchain_read: BlockchainReadHandle,
    range: Range<usize>,
    chain: Chain,
) -> Result<Vec<ExtendedBlockHeader>, Error> {
    let BlockchainResponse::BlockExtendedHeaderInRange(output) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockExtendedHeaderInRange(
            range, chain,
        ))
        .await?
    else {
        unreachable!();
    };

    Ok(output)
}

/// [`BlockchainReadRequest::ChainHeight`].
pub(super) async fn chain_height(
    mut blockchain_read: BlockchainReadHandle,
) -> Result<(u64, [u8; 32]), Error> {
    let BlockchainResponse::ChainHeight(height, hash) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::ChainHeight)
        .await?
    else {
        unreachable!();
    };

    Ok((usize_to_u64(height), hash))
}

/// [`BlockchainReadRequest::GeneratedCoins`].
pub(super) async fn generated_coins(
    mut blockchain_read: BlockchainReadHandle,
    block_height: u64,
) -> Result<u64, Error> {
    let BlockchainResponse::GeneratedCoins(generated_coins) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::GeneratedCoins(u64_to_usize(
            block_height,
        )))
        .await?
    else {
        unreachable!();
    };

    Ok(generated_coins)
}

/// [`BlockchainReadRequest::Outputs`]
pub(super) async fn outputs(
    mut blockchain_read: BlockchainReadHandle,
    outputs: HashMap<u64, HashSet<u64>>,
) -> Result<HashMap<u64, HashMap<u64, OutputOnChain>>, Error> {
    let BlockchainResponse::Outputs(outputs) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::Outputs(outputs))
        .await?
    else {
        unreachable!();
    };

    Ok(outputs)
}

/// [`BlockchainReadRequest::NumberOutputsWithAmount`]
pub(super) async fn number_outputs_with_amount(
    mut blockchain_read: BlockchainReadHandle,
    output_amounts: Vec<u64>,
) -> Result<HashMap<u64, usize>, Error> {
    let BlockchainResponse::NumberOutputsWithAmount(map) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::NumberOutputsWithAmount(
            output_amounts,
        ))
        .await?
    else {
        unreachable!();
    };

    Ok(map)
}

/// [`BlockchainReadRequest::KeyImagesSpent`]
pub(super) async fn key_images_spent(
    mut blockchain_read: BlockchainReadHandle,
    key_images: HashSet<[u8; 32]>,
) -> Result<bool, Error> {
    let BlockchainResponse::KeyImagesSpent(is_spent) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::KeyImagesSpent(key_images))
        .await?
    else {
        unreachable!();
    };

    Ok(is_spent)
}

/// [`BlockchainReadRequest::CompactChainHistory`]
pub(super) async fn compact_chain_history(
    mut blockchain_read: BlockchainReadHandle,
) -> Result<(Vec<[u8; 32]>, u128), Error> {
    let BlockchainResponse::CompactChainHistory {
        block_ids,
        cumulative_difficulty,
    } = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::CompactChainHistory)
        .await?
    else {
        unreachable!();
    };

    Ok((block_ids, cumulative_difficulty))
}

/// [`BlockchainReadRequest::FindFirstUnknown`]
pub(super) async fn find_first_unknown(
    mut blockchain_read: BlockchainReadHandle,
    hashes: Vec<[u8; 32]>,
) -> Result<Option<(usize, u64)>, Error> {
    let BlockchainResponse::FindFirstUnknown(resp) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::FindFirstUnknown(hashes))
        .await?
    else {
        unreachable!();
    };

    Ok(resp.map(|(index, height)| (index, usize_to_u64(height))))
}

/// [`BlockchainReadRequest::TotalTxCount`]
pub(super) async fn total_tx_count(
    mut blockchain_read: BlockchainReadHandle,
) -> Result<u64, Error> {
    let BlockchainResponse::TotalTxCount(tx_count) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::TotalTxCount)
        .await?
    else {
        unreachable!();
    };

    Ok(usize_to_u64(tx_count))
}

/// [`BlockchainReadRequest::DatabaseSize`]
pub(super) async fn database_size(
    mut blockchain_read: BlockchainReadHandle,
) -> Result<(u64, u64), Error> {
    let BlockchainResponse::DatabaseSize {
        database_size,
        free_space,
    } = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::DatabaseSize)
        .await?
    else {
        unreachable!();
    };

    Ok((database_size, free_space))
}

/// [`BlockchainReadRequest::Difficulty`]
pub(super) async fn difficulty(
    mut blockchain_read: BlockchainReadHandle,
    block_height: u64,
) -> Result<u128, Error> {
    let BlockchainResponse::Difficulty(difficulty) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::Difficulty(u64_to_usize(
            block_height,
        )))
        .await?
    else {
        unreachable!();
    };

    Ok(difficulty)
}

/// [`BlockchainReadRequest::OutputHistogram`]
pub(super) async fn output_histogram(
    mut blockchain_read: BlockchainReadHandle,
) -> Result<(), Error> {
    let BlockchainResponse::OutputHistogram(_) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::OutputHistogram)
        .await?
    else {
        unreachable!();
    };

    Ok(todo!())
}

/// [`BlockchainReadRequest::CoinbaseTxSum`]
pub(super) async fn coinbase_tx_sum(
    mut blockchain_read: BlockchainReadHandle,
) -> Result<(), Error> {
    let BlockchainResponse::CoinbaseTxSum(_) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::CoinbaseTxSum)
        .await?
    else {
        unreachable!();
    };

    Ok(todo!())
}

/// [`BlockchainReadRequest::MinerData`]
pub(super) async fn miner_data(mut blockchain_read: BlockchainReadHandle) -> Result<(), Error> {
    let BlockchainResponse::MinerData(_) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::MinerData)
        .await?
    else {
        unreachable!();
    };

    Ok(todo!())
}
