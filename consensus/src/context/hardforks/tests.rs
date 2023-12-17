use monero_consensus::hard_forks::{HFInfo, HardFork, NUMB_OF_HARD_FORKS};
use monero_consensus::HFsInfo;

use super::{HardForkConfig, HardForkState};
use crate::test_utils::mock_db::*;

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
        db_builder.finish(),
    )
    .await
    .unwrap();

    assert_eq!(state.current_hardfork, HardFork::V14);
}
