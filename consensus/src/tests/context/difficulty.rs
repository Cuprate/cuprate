use std::collections::VecDeque;

use proptest::{arbitrary::any, prop_assert_eq, prop_compose, proptest};

use cuprate_helper::num::median;

use crate::{context::difficulty::*, tests::mock_db::*, HardFork};

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

prop_compose! {
    /// Generates an arbitrary full difficulty cache.
    fn arb_full_difficulty_cache()
                           (
                               blocks in any::<[(u64, u64); TEST_TOTAL_ACCOUNTED_BLOCKS]>()
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

proptest! {
    #[test]
    fn check_calculations_lag(
        mut diff_cache in arb_full_difficulty_cache(),
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
    fn next_difficulty_consistant(diff_cache in arb_full_difficulty_cache(), hf in any::<HardFork>()) {
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
    fn window_size_kept_constant(mut diff_cache in arb_full_difficulty_cache(), new_blocks in any::<Vec<(u64, u128)>>()) {
        for (timestamp, cumulative_difficulty) in new_blocks.into_iter() {
            diff_cache.new_block(diff_cache.last_accounted_height+1, timestamp, cumulative_difficulty);
            prop_assert_eq!(diff_cache.timestamps.len(), TEST_TOTAL_ACCOUNTED_BLOCKS);
            prop_assert_eq!(diff_cache.cumulative_difficulties.len(), TEST_TOTAL_ACCOUNTED_BLOCKS);
        }
    }
}
