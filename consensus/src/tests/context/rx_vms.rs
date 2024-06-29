use std::collections::VecDeque;

use proptest::prelude::*;
use tokio::runtime::Builder;

use cuprate_consensus_rules::{
    blocks::{is_randomx_seed_height, randomx_seed_height},
    HardFork,
};

use crate::{
    context::rx_vms::{get_last_rx_seed_heights, RandomXVMCache},
    tests::mock_db::*,
};

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
async fn rx_vm_created_on_hf_12() {
    let db = DummyDatabaseBuilder::default().finish(Some(10));

    let mut cache = RandomXVMCache::init_from_chain_height(10, &HardFork::V11, db)
        .await
        .unwrap();

    assert!(cache.vms.is_empty());
    cache.new_block(11, &[30; 32]);
    cache.get_vms().await;

    assert!(!cache.vms.is_empty());
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
            let cache = RandomXVMCache::init_from_chain_height(10, &hf, db).await.unwrap();
            assert!(cache.seeds.len() == cache.vms.len() || hf < HardFork::V12);
        });
    }
}
