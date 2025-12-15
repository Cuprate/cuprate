//! Functions to send [`BlockchainReadRequest`]s.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ops::Range,
};

use anyhow::Error;
use indexmap::{IndexMap, IndexSet};
use monero_oxide::block::Block;
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_rpc_types::misc::GetOutputsOut;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    output_cache::OutputCache,
    rpc::{
        ChainInfo, CoinbaseTxSum, KeyImageSpentStatus, OutputDistributionData,
        OutputHistogramEntry, OutputHistogramInput,
    },
    BlockCompleteEntry, Chain, ExtendedBlockHeader, OutputDistributionInput, OutputOnChain,
    TxInBlockchain,
};

/// [`BlockchainReadRequest::Block`].
pub async fn block(
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
pub async fn block_by_hash(
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
pub async fn block_extended_header(
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
pub async fn block_hash(
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
pub async fn find_block(
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

/// [`BlockchainReadRequest::NextChainEntry`].
///
/// Returns only the:
/// - block IDs
/// - start height
/// - current chain height
pub async fn next_chain_entry(
    blockchain_read: &mut BlockchainReadHandle,
    block_hashes: Vec<[u8; 32]>,
    start_height: u64,
) -> Result<(Vec<[u8; 32]>, Option<usize>, usize), Error> {
    let BlockchainResponse::NextChainEntry {
        block_ids,
        start_height,
        chain_height,
        ..
    } = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::NextChainEntry(block_hashes, 500))
        .await?
    else {
        unreachable!();
    };

    Ok((block_ids, start_height, chain_height))
}

/// [`BlockchainReadRequest::FilterUnknownHashes`].
pub async fn filter_unknown_hashes(
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
pub async fn block_extended_header_in_range(
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
pub async fn chain_height(
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
pub async fn generated_coins(
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
pub async fn outputs(
    blockchain_read: &mut BlockchainReadHandle,
    outputs: IndexMap<u64, IndexSet<u64>>,
    get_txid: bool,
) -> Result<OutputCache, Error> {
    let BlockchainResponse::Outputs(outputs) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::Outputs { outputs, get_txid })
        .await?
    else {
        unreachable!();
    };

    Ok(outputs)
}

/// [`BlockchainReadRequest::OutputsVec`]
pub async fn outputs_vec(
    blockchain_read: &mut BlockchainReadHandle,
    outputs: Vec<GetOutputsOut>,
    get_txid: bool,
) -> Result<Vec<(u64, Vec<(u64, OutputOnChain)>)>, Error> {
    let outputs = outputs
        .into_iter()
        .map(|output| (output.amount, output.index))
        .collect();

    let BlockchainResponse::OutputsVec(outputs) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::OutputsVec { outputs, get_txid })
        .await?
    else {
        unreachable!();
    };

    Ok(outputs)
}

/// [`BlockchainReadRequest::NumberOutputsWithAmount`]
pub async fn number_outputs_with_amount(
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
pub async fn key_images_spent(
    blockchain_read: &mut BlockchainReadHandle,
    key_images: HashSet<[u8; 32]>,
) -> Result<bool, Error> {
    let BlockchainResponse::KeyImagesSpent(status) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::KeyImagesSpent(key_images))
        .await?
    else {
        unreachable!();
    };

    Ok(status)
}

/// [`BlockchainReadRequest::KeyImagesSpentVec`]
pub async fn key_images_spent_vec(
    blockchain_read: &mut BlockchainReadHandle,
    key_images: Vec<[u8; 32]>,
) -> Result<Vec<bool>, Error> {
    let BlockchainResponse::KeyImagesSpentVec(status) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::KeyImagesSpentVec(key_images))
        .await?
    else {
        unreachable!();
    };

    Ok(status)
}

/// [`BlockchainReadRequest::CompactChainHistory`]
pub async fn compact_chain_history(
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
pub async fn find_first_unknown(
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
pub async fn total_tx_count(blockchain_read: &mut BlockchainReadHandle) -> Result<u64, Error> {
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
pub async fn database_size(
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

/// [`BlockchainReadRequest::OutputDistribution`]
pub async fn output_distribution(
    blockchain_read: &mut BlockchainReadHandle,
    input: OutputDistributionInput,
) -> Result<Vec<OutputDistributionData>, Error> {
    let BlockchainResponse::OutputDistribution(data) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::OutputDistribution(input))
        .await?
    else {
        unreachable!();
    };

    Ok(data)
}

/// [`BlockchainReadRequest::OutputHistogram`]
pub async fn output_histogram(
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
pub async fn coinbase_tx_sum(
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
pub async fn alt_chains(
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
pub async fn alt_chain_count(blockchain_read: &mut BlockchainReadHandle) -> Result<u64, Error> {
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

/// [`BlockchainReadRequest::Transactions`].
pub async fn transactions(
    blockchain_read: &mut BlockchainReadHandle,
    tx_hashes: HashSet<[u8; 32]>,
) -> Result<(Vec<TxInBlockchain>, Vec<[u8; 32]>), Error> {
    let BlockchainResponse::Transactions { txs, missed_txs } = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::Transactions { tx_hashes })
        .await?
    else {
        unreachable!();
    };

    Ok((txs, missed_txs))
}

/// [`BlockchainReadRequest::TotalRctOutputs`].
pub async fn total_rct_outputs(blockchain_read: &mut BlockchainReadHandle) -> Result<u64, Error> {
    let BlockchainResponse::TotalRctOutputs(n) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::TotalRctOutputs)
        .await?
    else {
        unreachable!();
    };

    Ok(n)
}

/// [`BlockchainReadRequest::BlockCompleteEntries`].
pub async fn block_complete_entries(
    blockchain_read: &mut BlockchainReadHandle,
    block_hashes: Vec<[u8; 32]>,
) -> Result<(Vec<BlockCompleteEntry>, Vec<[u8; 32]>, usize), Error> {
    let BlockchainResponse::BlockCompleteEntries {
        blocks,
        output_indices: _,
        missing_hashes,
        blockchain_height,
    } = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockCompleteEntries(block_hashes))
        .await?
    else {
        unreachable!();
    };

    Ok((blocks, missing_hashes, blockchain_height))
}

/// [`BlockchainReadRequest::BlockCompleteEntriesAboveSplitPoint`].
pub async fn block_complete_entries_above_split_point(
    blockchain_read: &mut BlockchainReadHandle,
    chain: Vec<[u8; 32]>,
    get_indices: bool,
    pruned: bool,
) -> Result<
    (
        Vec<BlockCompleteEntry>,
        usize,
        usize,
        Vec<Vec<Vec<u64>>>,
    ),
    Error,
> {
    let BlockchainResponse::BlockCompleteEntriesAboveSplitPoint {
        blocks,
        output_indices,
        blockchain_height,
        start_height,
    } = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockCompleteEntriesAboveSplitPoint {
            chain,
            get_indices,
            len: 1000,
            pruned,
        })
        .await?
    else {
        unreachable!();
    };

    Ok((blocks, blockchain_height,start_height, output_indices))
}

/// [`BlockchainReadRequest::BlockCompleteEntriesByHeight`].
pub async fn block_complete_entries_by_height(
    blockchain_read: &mut BlockchainReadHandle,
    block_heights: Vec<u64>,
) -> Result<Vec<BlockCompleteEntry>, Error> {
    let BlockchainResponse::BlockCompleteEntriesByHeight(blocks) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockCompleteEntriesByHeight(
            block_heights.into_iter().map(u64_to_usize).collect(),
        ))
        .await?
    else {
        unreachable!();
    };

    Ok(blocks)
}

/// [`BlockchainReadRequest::TxOutputIndexes`].
pub async fn tx_output_indexes(
    blockchain_read: &mut BlockchainReadHandle,
    tx_hash: [u8; 32],
) -> Result<Vec<u64>, Error> {
    let BlockchainResponse::TxOutputIndexes(o_indexes) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::TxOutputIndexes { tx_hash })
        .await?
    else {
        unreachable!();
    };

    Ok(o_indexes)
}
