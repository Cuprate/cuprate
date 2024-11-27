//! Functions to send [`BlockchainReadRequest`]s.

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use anyhow::Error;
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain, ChainInfo, CoinbaseTxSum, ExtendedBlockHeader, OutputHistogramEntry,
    OutputHistogramInput, OutputOnChain,
};

/// [`BlockchainReadRequest::Block`].
pub(crate) async fn block(
    blockchain_read: &mut BlockchainReadHandle,
    height: u64,
) -> Result<Block, Error> {
    let BlockchainResponse::Block(block) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::Block {
            height: u64_to_usize(height),
        })
        .await?
    else {
        unreachable!();
    };

    Ok(block)
}

/// [`BlockchainReadRequest::BlockByHash`].
pub(crate) async fn block_by_hash(
    blockchain_read: &mut BlockchainReadHandle,
    hash: [u8; 32],
) -> Result<Block, Error> {
    let BlockchainResponse::Block(block) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockByHash(hash))
        .await?
    else {
        unreachable!();
    };

    Ok(block)
}

/// [`BlockchainReadRequest::BlockExtendedHeader`].
pub(crate) async fn block_extended_header(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn block_hash(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn find_block(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn filter_unknown_hashes(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn block_extended_header_in_range(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn chain_height(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn generated_coins(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn outputs(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn number_outputs_with_amount(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn key_images_spent(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn compact_chain_history(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn find_first_unknown(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn total_tx_count(
    blockchain_read: &mut BlockchainReadHandle,
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
pub(crate) async fn database_size(
    blockchain_read: &mut BlockchainReadHandle,
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

/// [`BlockchainReadRequest::OutputHistogram`]
pub(crate) async fn output_histogram(
    blockchain_read: &mut BlockchainReadHandle,
    input: OutputHistogramInput,
) -> Result<Vec<OutputHistogramEntry>, Error> {
    let BlockchainResponse::OutputHistogram(histogram) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::OutputHistogram(input))
        .await?
    else {
        unreachable!();
    };

    Ok(histogram)
}

/// [`BlockchainReadRequest::CoinbaseTxSum`]
pub(crate) async fn coinbase_tx_sum(
    blockchain_read: &mut BlockchainReadHandle,
    height: u64,
    count: u64,
) -> Result<CoinbaseTxSum, Error> {
    let BlockchainResponse::CoinbaseTxSum(sum) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::CoinbaseTxSum {
            height: u64_to_usize(height),
            count,
        })
        .await?
    else {
        unreachable!();
    };

    Ok(sum)
}

/// [`BlockchainReadRequest::AltChains`]
pub(crate) async fn alt_chains(
    blockchain_read: &mut BlockchainReadHandle,
) -> Result<Vec<ChainInfo>, Error> {
    let BlockchainResponse::AltChains(vec) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::AltChains)
        .await?
    else {
        unreachable!();
    };

    Ok(vec)
}

/// [`BlockchainReadRequest::AltChainCount`]
pub(crate) async fn alt_chain_count(
    blockchain_read: &mut BlockchainReadHandle,
) -> Result<u64, Error> {
    let BlockchainResponse::AltChainCount(count) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::AltChainCount)
        .await?
    else {
        unreachable!();
    };

    Ok(usize_to_u64(count))
}
