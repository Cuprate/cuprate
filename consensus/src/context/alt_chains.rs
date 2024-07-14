use std::collections::HashMap;

use tower::{Service, ServiceExt};

use cuprate_consensus_rules::{blocks::BlockError, ConsensusError, HardFork};
use cuprate_types::blockchain::{BCReadRequest, BCResponse, Chain, ChainID};

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
    weight_cache: Option<BlockWeightsCache>,
    difficulty_cache: Option<DifficultyCache>,

    chain_height: Option<u64>,
    top_hash: Option<[u8; 32]>,
    chain_id: Option<ChainID>,
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

    pub fn get_alt_chain_context(&mut self, prev_id: [u8; 32]) -> AltChainContextCache {
        self.alt_cache_map
            .remove(&prev_id)
            .unwrap_or(AltChainContextCache {
                weight_cache: None,
                difficulty_cache: None,
                chain_height: None,
                top_hash: None,
                chain_id: None,
            })
    }
}

pub async fn get_alt_chain_difficulty_cache<D: Database>(
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
                    database.clone(),
                )
                .await?;

            difficulty_cache
        }
        chain @ Chain::Alt(_) => {
            // prev_id is in an alt chain, completely rebuild the cache.
            let difficulty_cache = DifficultyCache::init_from_chain_height(
                top_height + 1,
                main_chain_difficulty_cache.config,
                database.clone(),
                chain,
            )
            .await?;

            difficulty_cache
        }
    })
}

pub async fn get_alt_chain_weight_cache<D: Database>(
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
                .pop_blocks_main_chain(weight_cache.tip_height - top_height, database.clone())
                .await?;

            weight_cache
        }
        chain @ Chain::Alt(_) => {
            // prev_id is in an alt chain, completely rebuild the cache.
            let weight_cache = BlockWeightsCache::init_from_chain_height(
                top_height + 1,
                main_chain_weight_cache.config,
                database.clone(),
                chain,
            )
            .await?;

            weight_cache
        }
    })
}
