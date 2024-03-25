use proptest::strategy::ValueTree;
use proptest::{strategy::Strategy, test_runner::TestRunner};
use tower::ServiceExt;

use crate::{
    context::{
        initialize_blockchain_context, BlockChainContextRequest, BlockChainContextResponse,
        ContextConfig, NewBlockData,
    },
    tests::mock_db::*,
    HardFork,
};

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
    const BLOCKCHAIN_HEIGHT: u64 = 6000;

    let mut runner = TestRunner::default();
    let db = arb_dummy_database(BLOCKCHAIN_HEIGHT.try_into().unwrap())
        .new_tree(&mut runner)
        .unwrap()
        .current();

    let ctx_svc = initialize_blockchain_context(TEST_CONTEXT_CONFIG, db).await?;

    let BlockChainContextResponse::Context(context) = ctx_svc
        .clone()
        .oneshot(BlockChainContextRequest::GetContext)
        .await?
    else {
        panic!("Context service returned wrong response!");
    };

    assert!(context.is_still_valid());
    assert!(context.is_still_valid());
    assert!(context.is_still_valid());

    ctx_svc
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

    assert!(!context.is_still_valid());

    Ok(())
}

#[tokio::test]
async fn context_height_correct() -> Result<(), tower::BoxError> {
    const BLOCKCHAIN_HEIGHT: u64 = 6000;

    let mut runner = TestRunner::default();
    let db = arb_dummy_database(BLOCKCHAIN_HEIGHT.try_into().unwrap())
        .new_tree(&mut runner)
        .unwrap()
        .current();

    let ctx_svc = initialize_blockchain_context(TEST_CONTEXT_CONFIG, db).await?;

    let BlockChainContextResponse::Context(context) = ctx_svc
        .oneshot(BlockChainContextRequest::GetContext)
        .await?
    else {
        panic!("context service returned incorrect response!")
    };

    assert_eq!(
        context.blockchain_context().unwrap().chain_height,
        BLOCKCHAIN_HEIGHT
    );

    Ok(())
}
