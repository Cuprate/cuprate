use cuprate_consensus_rules::hard_forks::{HFInfo, HFsInfo, HardFork, NUMB_OF_HARD_FORKS};

use crate::{
    context::{hardforks::HardForkState, HardForkConfig},
    tests::{
        context::data::{HFS_2678808_2688888, HFS_2688888_2689608},
        mock_db::*,
    },
};

const TEST_WINDOW_SIZE: u64 = 25;

const TEST_HFS: [HFInfo; NUMB_OF_HARD_FORKS] = [
    HFInfo::new(0, 0),
    HFInfo::new(10, 0),
    HFInfo::new(20, 0),
    HFInfo::new(30, 0),
    HFInfo::new(40, 0),
    HFInfo::new(50, 0),
    HFInfo::new(60, 0),
    HFInfo::new(70, 0),
    HFInfo::new(80, 0),
    HFInfo::new(90, 0),
    HFInfo::new(100, 0),
    HFInfo::new(110, 0),
    HFInfo::new(120, 0),
    HFInfo::new(130, 0),
    HFInfo::new(140, 0),
    HFInfo::new(150, 0),
];

pub const TEST_HARD_FORK_CONFIG: HardForkConfig = HardForkConfig {
    window: TEST_WINDOW_SIZE,
    info: HFsInfo::new(TEST_HFS),
};

#[tokio::test]
async fn hard_fork_set_depends_on_top_block() {
    let mut db_builder = DummyDatabaseBuilder::default();

    for _ in 0..TEST_WINDOW_SIZE {
        db_builder.add_block(
            DummyBlockExtendedHeader::default().with_hard_fork_info(HardFork::V13, HardFork::V16),
        );
    }
    db_builder.add_block(
        DummyBlockExtendedHeader::default().with_hard_fork_info(HardFork::V14, HardFork::V16),
    );

    let state = HardForkState::init_from_chain_height(
        TEST_WINDOW_SIZE + 1,
        TEST_HARD_FORK_CONFIG,
        db_builder.finish(None),
    )
    .await
    .unwrap();

    assert_eq!(state.current_hardfork, HardFork::V14);
}

#[tokio::test]
async fn hf_v15_v16_correct() {
    let mut db_builder = DummyDatabaseBuilder::default();

    for (version, vote) in HFS_2678808_2688888 {
        db_builder
            .add_block(DummyBlockExtendedHeader::default().with_hard_fork_info(version, vote));
    }

    let mut state = HardForkState::init_from_chain_height(
        2688888,
        HardForkConfig::main_net(),
        db_builder.finish(Some(2688888)),
    )
    .await
    .unwrap();

    for (i, (_, vote)) in HFS_2688888_2689608.into_iter().enumerate() {
        assert_eq!(state.current_hardfork, HardFork::V15);
        state.new_block(vote, (2688888 + i) as u64);
    }

    assert_eq!(state.current_hardfork, HardFork::V16);
}
