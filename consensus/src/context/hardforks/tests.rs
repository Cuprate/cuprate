use std::convert::TryInto;

use proptest::{arbitrary::any, prop_assert_eq, prop_compose, proptest};

use super::{HFInfo, HFVotes, HardFork, HardForkConfig, HardForkState, NUMB_OF_HARD_FORKS};
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
    forks: TEST_HFS,
};

#[test]
fn next_hard_forks() {
    let mut prev = HardFork::V1;
    let mut next = HardFork::V2;
    for _ in 2..NUMB_OF_HARD_FORKS {
        assert!(prev < next);
        prev = next;
        next = next.next_fork().unwrap();
    }
}

#[test]
fn hard_forks_defined() {
    for fork in 1..=NUMB_OF_HARD_FORKS {
        HardFork::from_version(&fork.try_into().unwrap()).unwrap();
    }
}

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

prop_compose! {
    /// Generates an arbitrary full [`HFVotes`].
    fn arb_full_hf_votes()
                   (
                       // we can't use HardFork as for some reason it overflows the stack, so we use u8.
                       votes in any::<[u8; TEST_WINDOW_SIZE as usize]>()
                   ) -> HFVotes {
        let mut vote_count = HFVotes::new(TEST_WINDOW_SIZE as usize);
        for vote in votes {
            vote_count.add_vote_for_hf(&HardFork::from_vote(&(vote % 17)));
        }
        vote_count
    }
}

proptest! {
    #[test]
    fn hf_vote_counter_total_correct(hf_votes in arb_full_hf_votes()) {
        prop_assert_eq!(hf_votes.total_votes(), u64::try_from(hf_votes.vote_list.len()).unwrap());

        let mut votes = [0_u64; NUMB_OF_HARD_FORKS];
        for vote in hf_votes.vote_list.iter() {
            // manually go through the list of votes tallying
            votes[*vote as usize - 1] += 1;
        }

        prop_assert_eq!(votes, hf_votes.votes);
    }

    #[test]
    fn window_size_kept_constant(mut hf_votes in arb_full_hf_votes(), new_votes in any::<Vec<HardFork>>()) {
        for new_vote in new_votes.into_iter() {
            hf_votes.add_vote_for_hf(&new_vote);
            prop_assert_eq!(hf_votes.total_votes(), TEST_WINDOW_SIZE)
        }
    }

    #[test]
    fn votes_out_of_range(high_vote in (NUMB_OF_HARD_FORKS+ 1).try_into().unwrap()..u8::MAX) {
        prop_assert_eq!(HardFork::from_vote(&0), HardFork::V1);
        prop_assert_eq!(HardFork::from_vote(&NUMB_OF_HARD_FORKS.try_into().unwrap()), HardFork::from_vote(&high_vote));
    }
}
