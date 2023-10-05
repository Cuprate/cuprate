use futures::join;
use monero_serai::{block::Block, transaction::Transaction};
use tower::ServiceExt;
use tracing::instrument;

use crate::{
    block::{pow::difficulty::DifficultyCache, weight::BlockWeightsCache},
    hardforks::{HardForkConfig, HardForkState},
    ConsensusError, Database, DatabaseRequest, DatabaseResponse,
};

pub struct Config {
    hard_fork_cfg: HardForkConfig,
}

impl Config {
    pub fn main_net() -> Config {
        Config {
            hard_fork_cfg: HardForkConfig::main_net(),
        }
    }
}

#[derive(Clone)]
struct State {
    block_weight: BlockWeightsCache,
    difficulty: DifficultyCache,
    hard_fork: HardForkState,
    chain_height: u64,
    top_hash: [u8; 32],
}

impl State {
    pub async fn init<D: Database + Clone>(
        config: Config,
        mut database: D,
    ) -> Result<State, ConsensusError> {
        let DatabaseResponse::ChainHeight(chain_height) = database
            .ready()
            .await?
            .call(DatabaseRequest::ChainHeight)
            .await?
        else {
            panic!("Database sent incorrect response")
        };

        Self::init_at_chain_height(config, chain_height, database).await
    }

    #[instrument(name = "init_state", skip_all)]
    pub async fn init_at_chain_height<D: Database + Clone>(
        config: Config,
        chain_height: u64,
        mut database: D,
    ) -> Result<State, ConsensusError> {
        let DatabaseResponse::BlockHash(top_hash) = database
            .ready()
            .await?
            .call(DatabaseRequest::BlockHash(chain_height - 1))
            .await?
        else {
            panic!("Database sent incorrect response")
        };

        let (block_weight, difficulty, hard_fork) = join!(
            BlockWeightsCache::init_from_chain_height(chain_height, database.clone()),
            DifficultyCache::init_from_chain_height(chain_height, database.clone()),
            HardForkState::init_from_chain_height(config.hard_fork_cfg, chain_height, database)
        );

        Ok(State {
            block_weight: block_weight?,
            difficulty: difficulty?,
            hard_fork: hard_fork?,
            chain_height,
            top_hash,
        })
    }
}

pub struct Verifier {
    state: State,
}

impl Verifier {
    pub async fn init<D: Database + Clone>(
        config: Config,
        mut database: D,
    ) -> Result<Verifier, ConsensusError> {
        let DatabaseResponse::ChainHeight(chain_height) = database
            .ready()
            .await?
            .call(DatabaseRequest::ChainHeight)
            .await?
        else {
            panic!("Database sent incorrect response")
        };

        Self::init_at_chain_height(config, chain_height, database).await
    }

    #[instrument(name = "init_verifier", skip_all)]
    pub async fn init_at_chain_height<D: Database + Clone>(
        config: Config,
        chain_height: u64,
        database: D,
    ) -> Result<Verifier, ConsensusError> {
        Ok(Verifier {
            state: State::init_at_chain_height(config, chain_height, database).await?,
        })
    }
}
