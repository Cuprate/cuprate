use proptest::strategy::ValueTree;
use proptest::{strategy::Strategy, test_runner::TestRunner};
use tower::ServiceExt;

use super::{
    difficulty::tests::TEST_DIFFICULTY_CONFIG, hardforks::tests::TEST_HARD_FORK_CONFIG,
    initialize_blockchain_context, weight::tests::TEST_WEIGHT_CONFIG, BlockChainContextRequest,
    ContextConfig, UpdateBlockchainCacheRequest,
};
use crate::{test_utils::mock_db::*, HardFork};

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

    let (ctx_svc, updater) = initialize_blockchain_context(TEST_CONTEXT_CONFIG, db).await?;

    let context = ctx_svc.oneshot(BlockChainContextRequest).await?;

    assert!(context.is_still_valid());
    assert!(context.is_still_valid());
    assert!(context.is_still_valid());

    updater
        .oneshot(UpdateBlockchainCacheRequest {
            new_top_hash: [0; 32],
            height: BLOCKCHAIN_HEIGHT,
            timestamp: 0,
            weight: 0,
            long_term_weight: 0,
            generated_coins: 0,
            vote: HardFork::V1,
            cumulative_difficulty: 0,
        })
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

    let (ctx_svc, _) = initialize_blockchain_context(TEST_CONTEXT_CONFIG, db).await?;

    let context = ctx_svc.oneshot(BlockChainContextRequest).await?;

    assert_eq!(
        context.blockchain_context().unwrap().chain_height,
        BLOCKCHAIN_HEIGHT
    );

    Ok(())
}
