use std::collections::VecDeque;

use proptest::collection::{size_range, vec};
use proptest::{prelude::*, prop_assert_eq, prop_compose, proptest};

use cuprate_helper::num::median;

use crate::{
    context::difficulty::*,
    tests::{context::data::DIF_3000000_3002000, mock_db::*},
    HardFork,
};

const TEST_WINDOW: usize = 72;
const TEST_CUT: usize = 6;
const TEST_LAG: usize = 2;

const TEST_TOTAL_ACCOUNTED_BLOCKS: usize = TEST_WINDOW + TEST_LAG;

pub const TEST_DIFFICULTY_CONFIG: DifficultyCacheConfig =
    DifficultyCacheConfig::new(TEST_WINDOW, TEST_CUT, TEST_LAG);

#[tokio::test]
async fn first_3_blocks_fixed_difficulty() -> Result<(), tower::BoxError> {
    let mut db_builder = DummyDatabaseBuilder::default();
    let genesis = DummyBlockExtendedHeader::default().with_difficulty_info(0, 1);
    db_builder.add_block(genesis);

    let mut difficulty_cache =
        DifficultyCache::init_from_chain_height(1, TEST_DIFFICULTY_CONFIG, db_builder.finish(None))
            .await?;

    for height in 1..3 {
        assert_eq!(difficulty_cache.next_difficulty(&HardFork::V1), 1);
        difficulty_cache.new_block(height, 0, u128::MAX);
    }
    Ok(())
}

#[tokio::test]
async fn genesis_block_skipped() -> Result<(), tower::BoxError> {
    let mut db_builder = DummyDatabaseBuilder::default();
    let genesis = DummyBlockExtendedHeader::default().with_difficulty_info(0, 1);
    db_builder.add_block(genesis);
    let diff_cache =
        DifficultyCache::init_from_chain_height(1, TEST_DIFFICULTY_CONFIG, db_builder.finish(None))
            .await?;
    assert!(diff_cache.cumulative_difficulties.is_empty());
    assert!(diff_cache.timestamps.is_empty());
    Ok(())
}

#[tokio::test]
async fn calculate_diff_3000000_3002000() -> Result<(), tower::BoxError> {
    let cfg = DifficultyCacheConfig::main_net();

    let mut db_builder = DummyDatabaseBuilder::default();
    for (cum_dif, timestamp) in DIF_3000000_3002000
        .iter()
        .take(cfg.total_block_count() as usize)
    {
        db_builder.add_block(
            DummyBlockExtendedHeader::default().with_difficulty_info(*timestamp, *cum_dif),
        )
    }

    let mut diff_cache = DifficultyCache::init_from_chain_height(
        3_000_720,
        cfg.clone(),
        db_builder.finish(Some(3_000_720)),
    )
    .await?;

    for (i, diff_info) in DIF_3000000_3002000
        .windows(2)
        .skip(cfg.total_block_count() as usize - 1)
        .enumerate()
    {
        let diff = diff_info[1].0 - diff_info[0].0;

        assert_eq!(diff_cache.next_difficulty(&HardFork::V16), diff);

        diff_cache.new_block(3_000_720 + i as u64, diff_info[1].1, diff_info[1].0);
    }

    Ok(())
}

prop_compose! {
    /// Generates an arbitrary full difficulty cache.
    fn arb_difficulty_cache(blocks: usize)
                           (
                               blocks in any_with::<Vec<(u64, u64)>>(size_range(blocks).lift()),
                           ) -> DifficultyCache {
        let (timestamps, mut cumulative_difficulties): (Vec<_>, Vec<_>) = blocks.into_iter().unzip();
        cumulative_difficulties.sort_unstable();
        DifficultyCache {
            last_accounted_height: timestamps.len().try_into().unwrap(),
            config: TEST_DIFFICULTY_CONFIG,
            timestamps: timestamps.into(),
            // we generate cumulative_difficulties in range 0..u64::MAX as if the generated values are close to u128::MAX
            // it will cause overflows
            cumulative_difficulties: cumulative_difficulties.into_iter().map(u128::from).collect(),
        }
    }
}

prop_compose! {
    fn random_difficulty_cache()(
        blocks in 0..TEST_TOTAL_ACCOUNTED_BLOCKS,
    )(
        diff_cache in arb_difficulty_cache(blocks)
    ) -> DifficultyCache {
        diff_cache
    }
}

proptest! {
    #[test]
    fn check_calculations_lag(
        mut diff_cache in arb_difficulty_cache(TEST_TOTAL_ACCOUNTED_BLOCKS),
        timestamp in any::<u64>(),
        cumulative_difficulty in any::<u128>(),
        hf in any::<HardFork>()
    ) {
        // duplicate the cache and remove the lag
        let mut no_lag_cache = diff_cache.clone();
        no_lag_cache.config.lag = 0;

        for _ in 0..TEST_LAG {
            // now remove the blocks that are outside our window due to no log
            no_lag_cache.timestamps.pop_front();
            no_lag_cache.cumulative_difficulties.pop_front();
        }
        // get the difficulty
        let next_diff_no_lag = no_lag_cache.next_difficulty(&hf);

        for _ in 0..TEST_LAG {
            // add new blocks to the lagged cache
            diff_cache.new_block(diff_cache.last_accounted_height+1, timestamp, cumulative_difficulty);
        }
        // they both should now be the same
        prop_assert_eq!(diff_cache.next_difficulty(&hf), next_diff_no_lag)
    }

    #[test]
    fn next_difficulty_consistent(diff_cache in arb_difficulty_cache(TEST_TOTAL_ACCOUNTED_BLOCKS), hf in any::<HardFork>()) {
        let first_call = diff_cache.next_difficulty(&hf);
        prop_assert_eq!(first_call, diff_cache.next_difficulty(&hf));
        prop_assert_eq!(first_call, diff_cache.next_difficulty(&hf));
        prop_assert_eq!(first_call, diff_cache.next_difficulty(&hf));
    }

    #[test]
    fn median_timestamp_adds_genesis(timestamps in any::<[u64; TEST_WINDOW -1]>()) {
        let mut timestamps: VecDeque<u64> = timestamps.into();

        let diff_cache = DifficultyCache {
            last_accounted_height: (TEST_WINDOW -1).try_into().unwrap(),
            config: TEST_DIFFICULTY_CONFIG,
            timestamps: timestamps.clone(),
            // we dont need cumulative_difficulties
            cumulative_difficulties: VecDeque::new(),
        };
        // add the genesis blocks timestamp (always 0)
        timestamps.push_front(0);
        timestamps.make_contiguous().sort_unstable();
        prop_assert_eq!(median(timestamps.make_contiguous()), diff_cache.median_timestamp(TEST_WINDOW).unwrap());
        // make sure adding the genesis block didn't persist
        prop_assert_eq!(diff_cache.timestamps.len(), TEST_WINDOW -1 );
    }

    #[test]
    fn window_size_kept_constant(mut diff_cache in arb_difficulty_cache(TEST_TOTAL_ACCOUNTED_BLOCKS), new_blocks in any::<Vec<(u64, u128)>>()) {
        for (timestamp, cumulative_difficulty) in new_blocks.into_iter() {
            diff_cache.new_block(diff_cache.last_accounted_height+1, timestamp, cumulative_difficulty);
            prop_assert_eq!(diff_cache.timestamps.len(), TEST_TOTAL_ACCOUNTED_BLOCKS);
            prop_assert_eq!(diff_cache.cumulative_difficulties.len(), TEST_TOTAL_ACCOUNTED_BLOCKS);
        }
    }

    #[test]
    fn claculating_multiple_diffs_does_not_change_state(
        diff_cache in random_difficulty_cache(),
        timestamps in any_with::<Vec<u64>>(size_range(0..1000).lift()),
        hf in any::<HardFork>(),
    ) {
        let cache = diff_cache.clone();

        diff_cache.next_difficulties(timestamps.into_iter().zip([hf].into_iter().cycle()).collect(), &hf);

        prop_assert_eq!(diff_cache, cache);
    }

    #[test]
    fn calculate_diff_in_advance(
        mut diff_cache in random_difficulty_cache(),
        timestamps in any_with::<Vec<u64>>(size_range(0..1000).lift()),
        hf in any::<HardFork>(),
    ) {
        let timestamps: Vec<_> = timestamps.into_iter().zip([hf].into_iter().cycle()).collect();

        let diffs = diff_cache.next_difficulties(timestamps.clone(), &hf);

        for (timestamp, diff) in timestamps.into_iter().zip(diffs.into_iter()) {
            prop_assert_eq!(diff_cache.next_difficulty(&timestamp.1), diff);
            diff_cache.new_block(diff_cache.last_accounted_height +1, timestamp.0, diff +  diff_cache.cumulative_difficulty());
        }

    }

    #[test]
    fn pop_blocks_below_total_blocks(
        mut database in arb_dummy_database(20),
        new_blocks in vec(any::<(u64, u128)>(), 0..500)
    ) {
        tokio_test::block_on(async move {
            let old_cache = DifficultyCache::init_from_chain_height(19, TEST_DIFFICULTY_CONFIG, database.clone()).await.unwrap();

            let blocks_to_pop = new_blocks.len();

            let mut new_cache = old_cache.clone();
            for (timestamp, cumulative_difficulty) in new_blocks.into_iter() {
                database.add_block(DummyBlockExtendedHeader::default().with_difficulty_info(timestamp, cumulative_difficulty));
                new_cache.new_block(new_cache.last_accounted_height+1, timestamp, cumulative_difficulty);
            }

            new_cache.pop_blocks(blocks_to_pop as u64, database).await?;

            prop_assert_eq!(new_cache, old_cache);

            Ok::<_, TestCaseError>(())
        })?;
    }

    #[test]
    fn pop_blocks_above_total_blocks(
        mut database in arb_dummy_database(2000),
        new_blocks in vec(any::<(u64, u128)>(), 0..5_000)
    ) {
        tokio_test::block_on(async move {
            let old_cache = DifficultyCache::init_from_chain_height(1999, TEST_DIFFICULTY_CONFIG, database.clone()).await.unwrap();

            let blocks_to_pop = new_blocks.len();

            let mut new_cache = old_cache.clone();
            for (timestamp, cumulative_difficulty) in new_blocks.into_iter() {
                database.add_block(DummyBlockExtendedHeader::default().with_difficulty_info(timestamp, cumulative_difficulty));
                new_cache.new_block(new_cache.last_accounted_height+1, timestamp, cumulative_difficulty);
            }

            new_cache.pop_blocks(blocks_to_pop as u64, database).await?;

            prop_assert_eq!(new_cache, old_cache);

            Ok::<_, TestCaseError>(())
        })?;
    }
}
