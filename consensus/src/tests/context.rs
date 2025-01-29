use proptest::strategy::ValueTree;
use proptest::{strategy::Strategy, test_runner::TestRunner};
use tower::ServiceExt;

use cuprate_consensus_context::{
    initialize_blockchain_context, BlockChainContextRequest, ContextConfig, NewBlockData,
};

use crate::{tests::mock_db::*, HardFork};

pub(crate) mod data;
mod difficulty;
mod hardforks;
mod rx_vms;
mod weight;

use difficulty::*;
use hardforks::*;
use weight::*;

const TEST_CONTEXT_CONFIG: ContextConfig = ContextConfig {
    hard_fork_cfg: TEST_HARD_FORK_CONFIG,
    difficulty_cfg: TEST_DIFFICULTY_CONFIG,
    weights_config: TEST_WEIGHT_CONFIG,
};

#[tokio::test]
async fn context_invalidated_on_new_block() -> Result<(), tower::BoxError> {
    const BLOCKCHAIN_HEIGHT: usize = 6000;

    let mut runner = TestRunner::default();
    let db = arb_dummy_database(BLOCKCHAIN_HEIGHT)
        .new_tree(&mut runner)
        .unwrap()
        .current();

    let mut ctx_svc = initialize_blockchain_context(TEST_CONTEXT_CONFIG, db).await?;

    let context = ctx_svc.blockchain_context().clone();

    ctx_svc
        .clone()
        .oneshot(BlockChainContextRequest::Update(NewBlockData {
            block_hash: [0; 32],
            height: BLOCKCHAIN_HEIGHT,
            timestamp: 0,
            weight: 0,
            long_term_weight: 0,
            generated_coins: 0,
            vote: HardFork::V1,
            cumulative_difficulty: 0,
        }))
        .await?;

    assert_ne!(&context, ctx_svc.blockchain_context());

    Ok(())
}

#[tokio::test]
async fn context_height_correct() -> Result<(), tower::BoxError> {
    const BLOCKCHAIN_HEIGHT: usize = 6000;

    let mut runner = TestRunner::default();
    let db = arb_dummy_database(BLOCKCHAIN_HEIGHT)
        .new_tree(&mut runner)
        .unwrap()
        .current();

    let mut ctx_svc = initialize_blockchain_context(TEST_CONTEXT_CONFIG, db).await?;

    let context = ctx_svc.blockchain_context();

    assert_eq!(context.chain_height, BLOCKCHAIN_HEIGHT);

    Ok(())
}
