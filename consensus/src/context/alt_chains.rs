use std::collections::HashMap;
use std::sync::Arc;

use tower::{Service, ServiceExt};

use cuprate_consensus_rules::{blocks::BlockError, ConsensusError};
use cuprate_types::{
    blockchain::{BCReadRequest, BCResponse},
    Chain, ChainID,
};

use crate::context::rx_vms::RandomXVM;
use crate::{
    ExtendedConsensusError,
    __private::Database,
    context::{difficulty::DifficultyCache, weight::BlockWeightsCache},
};

pub(crate) mod sealed {
    #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
    pub struct AltChainRequestToken;
}

#[derive(Debug, Clone)]
pub struct AltChainContextCache {
    pub weight_cache: Option<BlockWeightsCache>,
    pub difficulty_cache: Option<DifficultyCache>,

    pub cached_rx_vm: Option<(u64, Arc<RandomXVM>)>,

    pub chain_height: u64,
    pub top_hash: [u8; 32],
    pub chain_id: Option<ChainID>,
    pub parent_chain: Chain,
}

impl AltChainContextCache {}

pub struct AltChainMap {
    alt_cache_map: HashMap<[u8; 32], AltChainContextCache>,
}

impl AltChainMap {
    pub fn new() -> AltChainMap {
        AltChainMap {
            alt_cache_map: HashMap::new(),
        }
    }

    pub async fn get_alt_chain_context<D: Database>(
        &mut self,
        prev_id: [u8; 32],
        database: D,
    ) -> Result<AltChainContextCache, ExtendedConsensusError> {
        if let Some(cache) = self.alt_cache_map.remove(&prev_id) {
            return Ok(cache);
        }

        // find the block with hash == prev_id.
        let BCResponse::FindBlock(res) =
            database.oneshot(BCReadRequest::FindBlock(prev_id)).await?
        else {
            panic!("Database returned wrong response");
        };

        let Some((parent_chain, top_height)) = res else {
            // Couldn't find prev_id
            Err(ConsensusError::Block(BlockError::PreviousIDIncorrect))?
        };

        Ok(AltChainContextCache {
            weight_cache: None,
            difficulty_cache: None,
            cached_rx_vm: None,
            chain_height: top_height,
            top_hash: prev_id,
            chain_id: None,
            parent_chain,
        })
    }
}

pub async fn get_alt_chain_difficulty_cache<D: Database + Clone>(
    prev_id: [u8; 32],
    main_chain_difficulty_cache: &DifficultyCache,
    mut database: D,
) -> Result<DifficultyCache, ExtendedConsensusError> {
    // find the block with hash == prev_id.
    let BCResponse::FindBlock(res) = database
        .ready()
        .await?
        .call(BCReadRequest::FindBlock(prev_id))
        .await?
    else {
        panic!("Database returned wrong response");
    };

    let Some((chain, top_height)) = res else {
        // Can't find prev_id
        Err(ConsensusError::Block(BlockError::PreviousIDIncorrect))?
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
        chain @ Chain::Alt(_) => {
            // prev_id is in an alt chain, completely rebuild the cache.
            let difficulty_cache = DifficultyCache::init_from_chain_height(
                top_height + 1,
                main_chain_difficulty_cache.config,
                database,
                chain,
            )
            .await?;

            difficulty_cache
        }
    })
}

pub async fn get_alt_chain_weight_cache<D: Database + Clone>(
    prev_id: [u8; 32],
    main_chain_weight_cache: &BlockWeightsCache,
    mut database: D,
) -> Result<BlockWeightsCache, ExtendedConsensusError> {
    // find the block with hash == prev_id.
    let BCResponse::FindBlock(res) = database
        .ready()
        .await?
        .call(BCReadRequest::FindBlock(prev_id))
        .await?
    else {
        panic!("Database returned wrong response");
    };

    let Some((chain, top_height)) = res else {
        // Can't find prev_id
        Err(ConsensusError::Block(BlockError::PreviousIDIncorrect))?
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
        chain @ Chain::Alt(_) => {
            // prev_id is in an alt chain, completely rebuild the cache.
            let weight_cache = BlockWeightsCache::init_from_chain_height(
                top_height + 1,
                main_chain_weight_cache.config,
                database,
                chain,
            )
            .await?;

            weight_cache
        }
    })
}
