use std::collections::VecDeque;

use proptest::prelude::*;
use tokio::runtime::Builder;

use cuprate_consensus_context::rx_vms::{get_last_rx_seed_heights, RandomXVmCache};
use cuprate_consensus_rules::{
    blocks::{is_randomx_seed_height, randomx_seed_height},
    HardFork,
};

use crate::tests::mock_db::*;

#[test]
fn rx_heights_consistent() {
    let mut last_rx_heights = VecDeque::new();

    for height in 0..100_000_000 {
        if is_randomx_seed_height(height) {
            last_rx_heights.push_front(height);
            if last_rx_heights.len() > 3 {
                last_rx_heights.pop_back();
            }
        }

        assert_eq!(
            get_last_rx_seed_heights(height, 3).as_slice(),
            last_rx_heights.make_contiguous()
        );
        if last_rx_heights.len() >= 3 {
            assert!(
                randomx_seed_height(height) == last_rx_heights[0]
                    || randomx_seed_height(height) == last_rx_heights[1]
            );
        }
    }
}

#[tokio::test]
#[expect(unused_qualifications, reason = "false positive in tokio macro")]
async fn rx_vm_created_on_hf_12() {
    let db = DummyDatabaseBuilder::default().finish(Some(10));

    let mut cache = RandomXVmCache::init_from_chain_height(10, &HardFork::V11, db)
        .await
        .unwrap();

    assert!(cache.vms.is_empty());
    cache.new_block(11, &[30; 32]);
    cache.get_vms().await;

    assert!(!cache.vms.is_empty());
}

#[tokio::test]
#[expect(unused_qualifications, reason = "false positive in tokio macro")]
async fn rx_vm_pop_blocks() {
    let db = DummyDatabaseBuilder::default().finish(Some(2_000_000));

    let cache = RandomXVmCache::init_from_chain_height(2_000_000, &HardFork::V16, db.clone())
        .await
        .unwrap();

    let mut cloned_cache = cache.clone();

    for i in 0..2_000 {
        cloned_cache.new_block(2_000_000 + i, &[0; 32]);
    }

    cloned_cache
        .pop_blocks_main_chain(1_999_999, db.clone())
        .await
        .unwrap();

    assert_eq!(cloned_cache.seeds, cache.seeds);

    let mut cloned_cache = cache.clone();

    for i in 0..5_000 {
        cloned_cache.new_block(2_000_000 + i, &[0; 32]);
    }

    cloned_cache
        .pop_blocks_main_chain(1_999_999, db)
        .await
        .unwrap();

    assert_eq!(cloned_cache.seeds, cache.seeds);
}

proptest! {
    // these tests are expensive, so limit cases.
    #![proptest_config(ProptestConfig {
        cases: 3, .. ProptestConfig::default()
    })]
    #[test]
    fn rx_vm_created_only_after_hf_12(
        hf in any::<HardFork>(),
    ) {
        let db =  DummyDatabaseBuilder::default().finish(Some(10));

        let rt = Builder::new_multi_thread().enable_all().build().unwrap();

        rt.block_on(async move {
            let cache = RandomXVmCache::init_from_chain_height(10, &hf, db).await.unwrap();
            assert!(cache.seeds.len() == cache.vms.len() || hf < HardFork::V12);
        });
    }
}
