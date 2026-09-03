//! Functions to send [`BlockchainReadRequest`]s.
use anyhow::Error;
use monero_oxide::block::Block;
use tower::{Service, ServiceExt};

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_rpc_types::misc::GetOutputsOut;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    rpc::{
        ChainInfo, CoinbaseTxSum, OutputDistributionData, OutputHistogramEntry,
        OutputHistogramInput,
    },
    BlockCompleteEntry, Chain, ExtendedBlockHeader, OutputOnChain, PreRctOutputDistributionInput,
    TxInBlockchain,
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

/// [`BlockchainReadRequest::ChainHeight`].
pub(crate) async fn chain_height(blockchain_read: &mut BlockchainReadHandle) -> Result<u64, Error> {
    let BlockchainResponse::ChainHeight(height, _) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::ChainHeight)
        .await?
    else {
        unreachable!();
    };

    Ok(usize_to_u64(height))
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

/// [`BlockchainReadRequest::NextChainEntry`].
///
/// Returns only the:
/// - block IDs
/// - start height
/// - current chain height
pub(crate) async fn next_chain_entry(
    blockchain_read: &mut BlockchainReadHandle,
    block_hashes: Vec<[u8; 32]>,
) -> Result<(Vec<[u8; 32]>, Option<usize>, usize), Error> {
    let BlockchainResponse::NextChainEntry {
        block_ids,
        start_height,
        chain_height,
        ..
    } = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::NextChainEntry(block_hashes, 10_000))
        .await?
    else {
        unreachable!();
    };

    Ok((block_ids, start_height, chain_height))
}

/// [`BlockchainReadRequest::OutputsVec`]
pub(crate) async fn outputs_vec(
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

/// [`BlockchainReadRequest::KeyImagesSpentVec`]
pub(crate) async fn key_images_spent_vec(
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

/// [`BlockchainReadRequest::PreRctOutputDistribution`]
pub(crate) async fn pre_rct_output_distribution(
    blockchain_read: &mut BlockchainReadHandle,
    input: PreRctOutputDistributionInput,
) -> Result<Vec<OutputDistributionData>, Error> {
    let BlockchainResponse::PreRctOutputDistribution(data) = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::PreRctOutputDistribution(input))
        .await?
    else {
        unreachable!();
    };

    Ok(data)
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

/// [`BlockchainReadRequest::Transactions`].
pub(crate) async fn transactions(
    blockchain_read: &mut BlockchainReadHandle,
    tx_hashes: Vec<[u8; 32]>,
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
pub(crate) async fn total_rct_outputs(
    blockchain_read: &mut BlockchainReadHandle,
) -> Result<u64, Error> {
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

/// [`BlockchainReadRequest::BlockCompleteEntriesAboveSplitPoint`].
///
/// Returns `(blocks, blockchain_height, start_height, output_indices, top_hash)`.
pub(crate) async fn block_complete_entries_above_split_point(
    blockchain_read: &mut BlockchainReadHandle,
    chain: Vec<[u8; 32]>,
    start_height: Option<usize>,
    no_miner_tx: bool,
    len: usize,
    pruned: bool,
) -> Result<
    (
        Vec<BlockCompleteEntry>,
        usize,
        usize,
        Vec<Vec<Vec<u64>>>,
        [u8; 32],
    ),
    Error,
> {
    let BlockchainResponse::BlockCompleteEntriesAboveSplitPoint {
        blocks,
        output_indices,
        blockchain_height,
        start_height,
        top_hash,
    } = blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockCompleteEntriesAboveSplitPoint {
            chain,
            start_height,
            no_miner_tx,
            len,
            pruned,
        })
        .await?
    else {
        unreachable!();
    };

    Ok((
        blocks,
        blockchain_height,
        start_height,
        output_indices,
        top_hash,
    ))
}

/// [`BlockchainReadRequest::BlockCompleteEntriesByHeight`].
pub(crate) async fn block_complete_entries_by_height(
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
pub(crate) async fn tx_output_indexes(
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
