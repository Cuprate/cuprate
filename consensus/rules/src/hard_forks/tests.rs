use std::convert::TryInto;

use proptest::{arbitrary::any, prop_assert_eq, prop_compose, proptest};

use crate::hard_forks::{HFVotes, HardFork, NUMB_OF_HARD_FORKS};

const TEST_WINDOW_SIZE: usize = 25;

#[test]
fn target_block_time() {
    assert_eq!(HardFork::V1.block_time().as_secs(), 60);
    assert_eq!(HardFork::V2.block_time().as_secs(), 120);
}

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
        HardFork::from_version(fork.try_into().unwrap()).unwrap();
    }
}

prop_compose! {
    /// Generates an arbitrary full [`HFVotes`].
    fn arb_full_hf_votes()
                   (
                       // we can't use HardFork as for some reason it overflows the stack, so we use u8.
                       votes in any::<[u8; TEST_WINDOW_SIZE]>()
                   ) -> HFVotes {
        let mut vote_count = HFVotes::new(TEST_WINDOW_SIZE);
        for vote in votes {
            vote_count.add_vote_for_hf(&HardFork::from_vote(vote % 17));
        }
        vote_count
    }
}

proptest! {
    #[test]
    fn hf_vote_counter_total_correct(hf_votes in arb_full_hf_votes()) {
        prop_assert_eq!(hf_votes.total_votes(), hf_votes.vote_list.len());

        let mut votes = [0_usize; NUMB_OF_HARD_FORKS];
        for vote in &hf_votes.vote_list {
            // manually go through the list of votes tallying
            votes[*vote as usize - 1] += 1;
        }

        prop_assert_eq!(votes, hf_votes.votes);
    }

    #[test]
    fn window_size_kept_constant(mut hf_votes in arb_full_hf_votes(), new_votes in any::<Vec<HardFork>>()) {
        for new_vote in new_votes {
            hf_votes.add_vote_for_hf(&new_vote);
            prop_assert_eq!(hf_votes.total_votes(), TEST_WINDOW_SIZE);
        }
    }

    #[test]
    fn votes_out_of_range(high_vote in (NUMB_OF_HARD_FORKS+ 1).try_into().unwrap()..u8::MAX) {
        prop_assert_eq!(HardFork::from_vote(0), HardFork::V1);
        prop_assert_eq!(HardFork::from_vote(u8::try_from(NUMB_OF_HARD_FORKS).unwrap() + 1_u8), HardFork::from_vote(high_vote));
    }
}
