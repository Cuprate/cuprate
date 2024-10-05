use std::{collections::HashMap, sync::Arc};

use tower::ServiceExt;

use cuprate_consensus_rules::{blocks::BlockError, ConsensusError};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain, ChainId,
};

use crate::{
    ExtendedConsensusError,
    __private::Database,
    context::{difficulty::DifficultyCache, rx_vms::RandomXVm, weight::BlockWeightsCache},
};

pub(crate) mod sealed {
    /// A token that should be hard to create from outside this crate.
    ///
    /// It is currently possible to safely create this from outside this crate, **DO NOT** rely on this
    /// as it will be broken once we find a way to completely seal this.
    #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
    pub struct AltChainRequestToken;
}

/// The context cache of an alternative chain.
#[derive(Debug, Clone)]
pub struct AltChainContextCache {
    /// The alt chain weight cache, [`None`] if it has not been built yet.
    pub weight_cache: Option<BlockWeightsCache>,
    /// The alt chain difficulty cache, [`None`] if it has not been built yet.
    pub difficulty_cache: Option<DifficultyCache>,

    /// A cached RX VM.
    pub cached_rx_vm: Option<(usize, Arc<RandomXVm>)>,

    /// The chain height of the alt chain.
    pub chain_height: usize,
    /// The top hash of the alt chain.
    pub top_hash: [u8; 32],
    /// The [`ChainID`] of the alt chain.
    pub chain_id: Option<ChainId>,
    /// The parent [`Chain`] of this alt chain.
    pub parent_chain: Chain,
}

impl AltChainContextCache {
    /// Add a new block to the cache.
    pub fn add_new_block(
        &mut self,
        height: usize,
        block_hash: [u8; 32],
        block_weight: usize,
        long_term_block_weight: usize,
        timestamp: u64,
    ) {
        if let Some(difficulty_cache) = &mut self.difficulty_cache {
            difficulty_cache.new_block(height, timestamp, difficulty_cache.cumulative_difficulty());
        }

        if let Some(weight_cache) = &mut self.weight_cache {
            weight_cache.new_block(height, block_weight, long_term_block_weight);
        }

        self.chain_height += 1;
        self.top_hash = block_hash;
    }
}

/// A map of top IDs to alt chains.
pub(crate) struct AltChainMap {
    alt_cache_map: HashMap<[u8; 32], Box<AltChainContextCache>>,
}

impl AltChainMap {
    pub(crate) fn new() -> Self {
        Self {
            alt_cache_map: HashMap::new(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.alt_cache_map.clear();
    }

    /// Add an alt chain cache to the map.
    pub(crate) fn add_alt_cache(
        &mut self,
        prev_id: [u8; 32],
        alt_cache: Box<AltChainContextCache>,
    ) {
        self.alt_cache_map.insert(prev_id, alt_cache);
    }

    /// Attempts to take an [`AltChainContextCache`] from the map, returning [`None`] if no cache is
    /// present.
    pub(crate) async fn get_alt_chain_context<D: Database>(
        &mut self,
        prev_id: [u8; 32],
        database: D,
    ) -> Result<Box<AltChainContextCache>, ExtendedConsensusError> {
        if let Some(cache) = self.alt_cache_map.remove(&prev_id) {
            return Ok(cache);
        }

        // find the block with hash == prev_id.
        let BlockchainResponse::FindBlock(res) = database
            .oneshot(BlockchainReadRequest::FindBlock(prev_id))
            .await?
        else {
            panic!("Database returned wrong response");
        };

        let Some((parent_chain, top_height)) = res else {
            // Couldn't find prev_id
            return Err(ConsensusError::Block(BlockError::PreviousIDIncorrect).into());
        };

        Ok(Box::new(AltChainContextCache {
            weight_cache: None,
            difficulty_cache: None,
            cached_rx_vm: None,
            chain_height: top_height,
            top_hash: prev_id,
            chain_id: None,
            parent_chain,
        }))
    }
}

/// Builds a [`DifficultyCache`] for an alt chain.
pub(crate) async fn get_alt_chain_difficulty_cache<D: Database + Clone>(
    prev_id: [u8; 32],
    main_chain_difficulty_cache: &DifficultyCache,
    mut database: D,
) -> Result<DifficultyCache, ExtendedConsensusError> {
    // find the block with hash == prev_id.
    let BlockchainResponse::FindBlock(res) = database
        .ready()
        .await?
        .call(BlockchainReadRequest::FindBlock(prev_id))
        .await?
    else {
        panic!("Database returned wrong response");
    };

    let Some((chain, top_height)) = res else {
        // Can't find prev_id
        return Err(ConsensusError::Block(BlockError::PreviousIDIncorrect).into());
    };

    Ok(match chain {
        Chain::Main => {
            // prev_id is in main chain, we can use the fast path and clone the main chain cache.
            let mut difficulty_cache = main_chain_difficulty_cache.clone();
            difficulty_cache
                .pop_blocks_main_chain(
                    difficulty_cache.last_accounted_height - top_height,
                    database,
                )
                .await?;

            difficulty_cache
        }
        Chain::Alt(_) => {
            // prev_id is in an alt chain, completely rebuild the cache.
            DifficultyCache::init_from_chain_height(
                top_height + 1,
                main_chain_difficulty_cache.config,
                database,
                chain,
            )
            .await?
        }
    })
}

/// Builds a [`BlockWeightsCache`] for an alt chain.
pub(crate) async fn get_alt_chain_weight_cache<D: Database + Clone>(
    prev_id: [u8; 32],
    main_chain_weight_cache: &BlockWeightsCache,
    mut database: D,
) -> Result<BlockWeightsCache, ExtendedConsensusError> {
    // find the block with hash == prev_id.
    let BlockchainResponse::FindBlock(res) = database
        .ready()
        .await?
        .call(BlockchainReadRequest::FindBlock(prev_id))
        .await?
    else {
        panic!("Database returned wrong response");
    };

    let Some((chain, top_height)) = res else {
        // Can't find prev_id
        return Err(ConsensusError::Block(BlockError::PreviousIDIncorrect).into());
    };

    Ok(match chain {
        Chain::Main => {
            // prev_id is in main chain, we can use the fast path and clone the main chain cache.
            let mut weight_cache = main_chain_weight_cache.clone();
            weight_cache
                .pop_blocks_main_chain(weight_cache.tip_height - top_height, database)
                .await?;

            weight_cache
        }
        Chain::Alt(_) => {
            // prev_id is in an alt chain, completely rebuild the cache.
            BlockWeightsCache::init_from_chain_height(
                top_height + 1,
                main_chain_weight_cache.config,
                database,
                chain,
            )
            .await?
        }
    })
}
