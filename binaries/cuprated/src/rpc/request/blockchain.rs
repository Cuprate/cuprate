//! Functions for [`BlockchainReadRequest`] and [`BlockchainWriteRequest`].

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
};

use anyhow::{anyhow, Error};
use futures::StreamExt;
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use cuprate_consensus::BlockchainResponse;
use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainWriteRequest},
    Chain, ExtendedBlockHeader, HardFork, OutputOnChain, VerifiedBlockInformation,
};

use crate::rpc::CupratedRpcHandlerState;

impl CupratedRpcHandlerState {
    /// [`BlockchainReadRequest::BlockExtendedHeader`].
    pub(super) async fn block_extended_header(
        &mut self,
        height: u64,
    ) -> Result<ExtendedBlockHeader, Error> {
        let BlockchainResponse::BlockExtendedHeader(header) = self
            .blockchain_read
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
        &mut self,
        height: u64,
        chain: Chain,
    ) -> Result<[u8; 32], Error> {
        let BlockchainResponse::BlockHash(hash) = self
            .blockchain_read
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
        &mut self,
        block_hash: [u8; 32],
    ) -> Result<Option<(Chain, usize)>, Error> {
        let BlockchainResponse::FindBlock(option) = self
            .blockchain_read
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
        &mut self,
        block_hashes: HashSet<[u8; 32]>,
    ) -> Result<HashSet<[u8; 32]>, Error> {
        let BlockchainResponse::FilterUnknownHashes(output) = self
            .blockchain_read
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
        &mut self,
        range: Range<usize>,
        chain: Chain,
    ) -> Result<Vec<ExtendedBlockHeader>, Error> {
        let BlockchainResponse::BlockExtendedHeaderInRange(output) = self
            .blockchain_read
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
    pub(super) async fn chain_height(&mut self) -> Result<(u64, [u8; 32]), Error> {
        let BlockchainResponse::ChainHeight(height, hash) = self
            .blockchain_read
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
    pub(super) async fn generated_coins(&mut self, block_height: u64) -> Result<u64, Error> {
        let BlockchainResponse::GeneratedCoins(generated_coins) = self
            .blockchain_read
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
        &mut self,
        outputs: HashMap<u64, HashSet<u64>>,
    ) -> Result<HashMap<u64, HashMap<u64, OutputOnChain>>, Error> {
        let BlockchainResponse::Outputs(outputs) = self
            .blockchain_read
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
        &mut self,
        output_amounts: Vec<u64>,
    ) -> Result<HashMap<u64, usize>, Error> {
        let BlockchainResponse::NumberOutputsWithAmount(map) = self
            .blockchain_read
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
        &mut self,
        key_images: HashSet<[u8; 32]>,
    ) -> Result<bool, Error> {
        let BlockchainResponse::KeyImagesSpent(is_spent) = self
            .blockchain_read
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
    pub(super) async fn compact_chain_history(&mut self) -> Result<(Vec<[u8; 32]>, u128), Error> {
        let BlockchainResponse::CompactChainHistory {
            block_ids,
            cumulative_difficulty,
        } = self
            .blockchain_read
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
        &mut self,
        hashes: Vec<[u8; 32]>,
    ) -> Result<Option<(usize, u64)>, Error> {
        let BlockchainResponse::FindFirstUnknown(resp) = self
            .blockchain_read
            .ready()
            .await?
            .call(BlockchainReadRequest::FindFirstUnknown(hashes))
            .await?
        else {
            unreachable!();
        };

        Ok(resp.map(|(index, height)| (index, usize_to_u64(height))))
    }

    //------------------------------------------------------------------------------------------ new

    // /// [`BlockchainReadRequest::Block`].
    // pub(super) async fn block(&mut self, height: u64) -> Result<Block, Error> {
    //     let BlockchainResponse::Block(block) = self
    //
    //         .blockchain_read
    //         .ready()
    //         .await?
    //         .call(BlockchainReadRequest::Block(u64_to_usize(height)))
    //         .await?
    //     else {
    //         unreachable!();
    //     };

    //     Ok(block)
    // }

    // /// [`BlockchainReadRequest::BlockByHash`].
    // pub(super) async fn block_by_hash(&mut self, hash: [u8; 32]) -> Result<Block, Error> {
    //     let BlockchainResponse::BlockByHash(block) = self
    //
    //         .blockchain_read
    //         .ready()
    //         .await?
    //         .call(BlockchainReadRequest::BlockByHash(hash))
    //         .await?
    //     else {
    //         unreachable!();
    //     };

    //     Ok(block)
    // }

    // /// [`BlockchainReadRequest::BlockExtendedHeaderByHash`].
    // pub(super) async fn block_extended_header_by_hash(
    //     &mut self,
    //     hash: [u8; 32],
    // ) -> Result<ExtendedBlockHeader, Error> {
    //     let BlockchainResponse::BlockExtendedHeaderByHash(header) = self
    //
    //         .blockchain_read
    //         .ready()
    //         .await?
    //         .call(BlockchainReadRequest::BlockExtendedHeaderByHash(hash))
    //         .await?
    //     else {
    //         unreachable!();
    //     };

    //     Ok(header)
    // }

    // /// [`BlockchainReadRequest::TopBlockFull`].
    // pub(super) async fn top_block_full(&mut self) -> Result<(Block, ExtendedBlockHeader), Error> {
    //     let BlockchainResponse::TopBlockFull(block, header) = self
    //
    //         .blockchain_read
    //         .ready()
    //         .await?
    //         .call(BlockchainReadRequest::TopBlockFull)
    //         .await?
    //     else {
    //         unreachable!();
    //     };

    //     Ok((block, header))
    // }

    // /// [`BlockchainReadRequest::CurrentHardFork`]
    // pub(super) async fn current_hard_fork(&mut self) -> Result<HardFork, Error> {
    //     let BlockchainResponse::CurrentHardFork(hard_fork) = self
    //
    //         .blockchain_read
    //         .ready()
    //         .await?
    //         .call(BlockchainReadRequest::CurrentHardFork)
    //         .await?
    //     else {
    //         unreachable!();
    //     };

    //     Ok(hard_fork)
    // }

    // /// [`BlockchainReadRequest::PopBlocks`]
    // pub(super) async fn pop_blocks(&mut self, nblocks: u64) -> Result<u64, Error> {
    //     let BlockchainResponse::PopBlocks(height) = self
    //
    //         .blockchain_write
    //         .ready()
    //         .await?
    //         .call(BlockchainWriteRequest::PopBlocks(nblocks))
    //         .await?
    //     else {
    //         unreachable!();
    //     };

    //     Ok(usize_to_u64(height))
    // }

    // /// [`BlockchainReadRequest::CumulativeBlockWeightLimit`]
    // pub(super) async fn cumulative_block_weight_limit(&mut self) -> Result<usize, Error> {
    //     let BlockchainResponse::CumulativeBlockWeightLimit(limit) = self
    //
    //         .blockchain_read
    //         .ready()
    //         .await?
    //         .call(BlockchainReadRequest::CumulativeBlockWeightLimit)
    //         .await?
    //     else {
    //         unreachable!();
    //     };

    //     Ok(limit)
    // }
}
