use crate::{
    tests::{context::data::BW_2850000_3050000, mock_db::*},
    HardFork,
};
use cuprate_consensus_context::{
    weight::{calculate_block_long_term_weight, BlockWeightsCache},
    BlockWeightsCacheConfig,
};
use cuprate_types::Chain;

pub(crate) const TEST_WEIGHT_CONFIG: BlockWeightsCacheConfig =
    BlockWeightsCacheConfig::new(100, 5000);

#[tokio::test]
async fn blocks_out_of_window_not_counted() -> Result<(), tower::BoxError> {
    let mut db_builder = DummyDatabaseBuilder::default();
    for weight in 1..=5000 {
        let block = DummyBlockExtendedHeader::default().with_weight_into(weight, weight);
        db_builder.add_block(block);
    }

    let mut weight_cache = BlockWeightsCache::init_from_chain_height(
        5000,
        TEST_WEIGHT_CONFIG,
        db_builder.finish(None),
        Chain::Main,
    )
    .await?;
    assert_eq!(weight_cache.median_long_term_weight(), 2500);
    assert_eq!(weight_cache.median_short_term_weight(), 4950);

    weight_cache.new_block(5000, 0, 0);
    weight_cache.new_block(5001, 0, 0);
    weight_cache.new_block(5002, 0, 0);

    // if blocks outside the window were not removed adding the blocks above would have pulled the median down.
    assert_eq!(weight_cache.median_long_term_weight(), 2500);
    assert_eq!(weight_cache.median_short_term_weight(), 4950);

    Ok(())
}

#[tokio::test]
async fn pop_blocks_greater_than_window() -> Result<(), tower::BoxError> {
    let mut db_builder = DummyDatabaseBuilder::default();
    for weight in 1..=5000 {
        let block = DummyBlockExtendedHeader::default().with_weight_into(weight, weight);
        db_builder.add_block(block);
    }

    let database = db_builder.finish(None);

    let mut weight_cache = BlockWeightsCache::init_from_chain_height(
        5000,
        TEST_WEIGHT_CONFIG,
        database.clone(),
        Chain::Main,
    )
    .await?;

    let old_cache = weight_cache.clone();

    weight_cache.new_block(5000, 0, 0);
    weight_cache.new_block(5001, 0, 0);
    weight_cache.new_block(5002, 0, 0);

    weight_cache
        .pop_blocks_main_chain(3, database)
        .await
        .unwrap();

    assert_eq!(weight_cache, old_cache);

    Ok(())
}

#[tokio::test]
async fn pop_blocks_less_than_window() -> Result<(), tower::BoxError> {
    let mut db_builder = DummyDatabaseBuilder::default();
    for weight in 1..=500 {
        let block = DummyBlockExtendedHeader::default().with_weight_into(weight, weight);
        db_builder.add_block(block);
    }

    let database = db_builder.finish(None);

    let mut weight_cache = BlockWeightsCache::init_from_chain_height(
        500,
        TEST_WEIGHT_CONFIG,
        database.clone(),
        Chain::Main,
    )
    .await?;

    let old_cache = weight_cache.clone();

    weight_cache.new_block(500, 0, 0);
    weight_cache.new_block(501, 0, 0);
    weight_cache.new_block(502, 0, 0);

    weight_cache
        .pop_blocks_main_chain(3, database)
        .await
        .unwrap();

    assert_eq!(weight_cache, old_cache);

    Ok(())
}

#[tokio::test]
async fn weight_cache_calculates_correct_median() -> Result<(), tower::BoxError> {
    let mut db_builder = DummyDatabaseBuilder::default();
    // add an initial block as otherwise this will panic.
    let block = DummyBlockExtendedHeader::default().with_weight_into(0, 0);
    db_builder.add_block(block);

    let mut weight_cache = BlockWeightsCache::init_from_chain_height(
        1,
        TEST_WEIGHT_CONFIG,
        db_builder.finish(None),
        Chain::Main,
    )
    .await?;

    for height in 1..=100 {
        weight_cache.new_block(height, height, height);

        assert_eq!(weight_cache.median_short_term_weight(), height / 2);
        assert_eq!(weight_cache.median_long_term_weight(), height / 2);
    }

    for height in 101..=5000 {
        weight_cache.new_block(height, height, height);

        assert_eq!(weight_cache.median_long_term_weight(), height / 2);
    }
    Ok(())
}

#[tokio::test]
async fn calc_bw_ltw_2850000_3050000() {
    let mut db_builder = DummyDatabaseBuilder::default();

    for (weight, ltw) in BW_2850000_3050000.iter().take(100_000) {
        let block = DummyBlockExtendedHeader::default().with_weight_into(*weight, *ltw);
        db_builder.add_block(block);
    }

    let mut weight_cache = BlockWeightsCache::init_from_chain_height(
        2950000,
        TEST_WEIGHT_CONFIG,
        db_builder.finish(Some(2950000)),
        Chain::Main,
    )
    .await
    .unwrap();

    for (i, (weight, ltw)) in BW_2850000_3050000.iter().skip(100_000).enumerate() {
        let calc_ltw = calculate_block_long_term_weight(
            HardFork::V16,
            *weight,
            weight_cache.median_long_term_weight(),
        );
        assert_eq!(calc_ltw, *ltw);
        weight_cache.new_block(2950000 + i, *weight, *ltw);
    }
}

// TODO: protests
