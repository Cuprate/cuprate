use pretty_assertions::assert_eq;

use crate::{
    PowerChallenge, PowerChallengeP2p, PowerChallengeRpc, check_difficulty,
    create_difficulty_scalar, verify_p2p, verify_rpc,
};

/// Test difficulty, real difficulty value is too high for debug builds.
const DIFF: u32 = 15;

struct TestDataEquix {
    challenge: &'static str,
    expected_solution: &'static str,
    expected_solution_count: usize,
    expected_scalar: u32,
}

#[derive(Debug)]
struct TestDataRpc {
    tx_prefix_hash: &'static str,
    recent_block_hash: &'static str,
    expected_nonce: u32,
    /// Bytes of [`PowerSolution::challenge`],
    /// _not_ the initial challenge construction.
    expected_challenge: &'static str,
    expected_solution: &'static str,
    expected_scalar: u32,
}

struct TestDataP2p {
    seed: u64,
    seed_top64: u64,
    expected_nonce: u32,
    /// Bytes of [`PowerSolution::challenge`],
    /// _not_ the initial challenge construction.
    expected_challenge: &'static str,
    expected_solution: &'static str,
    expected_scalar: u32,
}

const TEST_DATA_EQUIX: [TestDataEquix; 5] = [
    // test UTF8
    TestDataEquix {
        challenge: "よ、ひさしぶりだね。",
        expected_solution: "546658a95f6466ecc41b24dca5a5e8f5",
        expected_solution_count: 3,
        expected_scalar: 609012647,
    },
    TestDataEquix {
        challenge: "👋,🕒👉🕘.",
        expected_solution: "7854ba6c1c9bf7cc9354aed876ce64f4",
        expected_solution_count: 3,
        expected_scalar: 1651207227,
    },
    TestDataEquix {
        challenge: "Privacy is necessary for an open society in the electronic age.",
        expected_solution: "7d1467364825e586ae44b9e95ff388f3",
        expected_solution_count: 4,
        expected_scalar: 2074493700,
    },
    TestDataEquix {
        challenge: "We must defend our own privacy if we expect to have any.",
        expected_solution: "a330e6561142a57be57513c1095d46ff",
        expected_solution_count: 3,
        expected_scalar: 1892198895,
    },
    TestDataEquix {
        challenge: "We must come together and create systems which allow anonymous transactions to take place.",
        expected_solution: "ca1e0362d9252bbb85c62fcdf4ac68f6",
        expected_solution_count: 2,
        expected_scalar: 283799637,
    },
];

const TEST_DATA_RPC: [TestDataRpc; 3] = [
    TestDataRpc {
        tx_prefix_hash: "c01d4920b75c0cad3a75aa71d6aa73e3d90d0be3ac8da5f562b3fc101e74b57c",
        recent_block_hash: "77ff034133bdd86914c6e177563ee8b08af896dd2603b882e280762deab609c0",
        expected_nonce: 5,
        expected_challenge: "4d6f6e65726f20506f574552c01d4920b75c0cad3a75aa71d6aa73e3d90d0be3ac8da5f562b3fc101e74b57c77ff034133bdd86914c6e177563ee8b08af896dd2603b882e280762deab609c005000000",
        expected_solution: "6c81ba867f822ea88b14fe2ed027e1ee",
        expected_scalar: 259977672,
    },
    TestDataRpc {
        tx_prefix_hash: "17bac54d909964de0ed46eda755904b33fb42eead7ce015fbdde17fa6f0ec95f",
        recent_block_hash: "6d4c090582ed8cecfc8f8d90ddd8e6b7c8b39dd86c7e882078b670a7ba29b03f",
        expected_nonce: 24,
        expected_challenge: "4d6f6e65726f20506f57455217bac54d909964de0ed46eda755904b33fb42eead7ce015fbdde17fa6f0ec95f6d4c090582ed8cecfc8f8d90ddd8e6b7c8b39dd86c7e882078b670a7ba29b03f18000000",
        expected_solution: "6992d7cb29ae95dbc92f6b8d50e820ef",
        expected_scalar: 252939049,
    },
    TestDataRpc {
        tx_prefix_hash: "6dd6a8df16e052f53d51f5f76372ab0c14c60d748908c4589a90327bdc6498a1",
        recent_block_hash: "bc322459b35f5c58082d4193c8d6bf4f057aedd0823121f2ecbcb117276d13a2",
        expected_nonce: 1,
        expected_challenge: "4d6f6e65726f20506f5745526dd6a8df16e052f53d51f5f76372ab0c14c60d748908c4589a90327bdc6498a1bc322459b35f5c58082d4193c8d6bf4f057aedd0823121f2ecbcb117276d13a201000000",
        expected_solution: "19018e8d20beaeda149816cd74f33bfd",
        expected_scalar: 187745649,
    },
];

const TEST_DATA_P2P: [TestDataP2p; 3] = [
    TestDataP2p {
        seed: 0,
        seed_top64: 0,
        expected_nonce: 10,
        expected_challenge: "4d6f6e65726f20506f574552000000000000000000000000000000000f0000000a000000",
        expected_solution: "ad025bac4c7bb2dfcb4bb666cf2643e8",
        expected_scalar: 252557470,
    },
    TestDataP2p {
        seed: 1_589_356,
        seed_top64: 6700,
        expected_nonce: 0,
        expected_challenge: "4d6f6e65726f20506f5745526c401800000000002c1a0000000000000f00000000000000",
        expected_solution: "0d25ad67fb065baae91a0d29a31db9d8",
        expected_scalar: 50548387,
    },
    TestDataP2p {
        seed: u64::MAX,
        seed_top64: u64::MAX,
        expected_nonce: 4,
        expected_challenge: "4d6f6e65726f20506f574552ffffffffffffffffffffffffffffffff0f00000004000000",
        expected_solution: "3357a279712c70e3e26442d864282ef8",
        expected_scalar: 170469575,
    },
];

/// Sanity test Equi-X.
#[test]
fn equix() {
    for t in TEST_DATA_EQUIX {
        let s = equix::solve(t.challenge.as_bytes()).unwrap();
        let solution_count = s.len();
        let solution = s.first().unwrap();

        assert_eq!(t.expected_solution_count, solution_count);
        assert_eq!(t.expected_solution, hex::encode(solution.to_bytes()));

        let scalar = create_difficulty_scalar(t.challenge.as_bytes(), solution);
        assert_eq!(t.expected_scalar, scalar);
    }
}

#[test]
fn rpc() {
    for t in TEST_DATA_RPC {
        println!("{t:?}");

        let tx_prefix_hash = hex::decode(t.tx_prefix_hash).unwrap().try_into().unwrap();
        let recent_block_hash = hex::decode(t.recent_block_hash)
            .unwrap()
            .try_into()
            .unwrap();

        let c1 = PowerChallengeRpc::new_from_input((tx_prefix_hash, recent_block_hash, 0));
        let c2 = c1.as_ref();
        drop(equix::solve(c2).unwrap());

        let s = c1.solve(DIFF);

        assert_eq!(hex::encode(&s.challenge), t.expected_challenge);

        let h = hex::encode(s.solution.to_bytes());
        assert_eq!(h, t.expected_solution);

        assert_eq!(s.nonce, t.expected_nonce);

        let c3 = PowerChallengeRpc::new_from_input((
            tx_prefix_hash,
            recent_block_hash,
            t.expected_nonce,
        ));
        assert_eq!(hex::encode(c3.as_ref()), t.expected_challenge);

        let d = create_difficulty_scalar(&s.challenge, &s.solution);
        assert_eq!(d, t.expected_scalar);

        let last_difficulty_that_passes = u32::MAX / d;

        assert_eq!(true, check_difficulty(d, last_difficulty_that_passes));
        assert_eq!(false, check_difficulty(d, last_difficulty_that_passes + 1));

        assert!(verify_rpc(
            tx_prefix_hash,
            recent_block_hash,
            t.expected_nonce,
            &s.solution.to_bytes(),
            DIFF,
        ));
    }
}

#[test]
fn p2p() {
    for t in TEST_DATA_P2P {
        let c1 = PowerChallengeP2p::new_from_input((t.seed, t.seed_top64, DIFF, 0));
        let c2 = c1.as_ref();
        drop(equix::solve(c2).unwrap());

        let s = c1.solve(DIFF);

        assert_eq!(hex::encode(&s.challenge), t.expected_challenge);

        let h = hex::encode(s.solution.to_bytes());
        assert_eq!(h, t.expected_solution);

        assert_eq!(s.nonce, t.expected_nonce);

        let c3 = PowerChallengeP2p::new_from_input((t.seed, t.seed_top64, DIFF, t.expected_nonce));
        assert_eq!(hex::encode(c3.as_ref()), t.expected_challenge);

        let d = create_difficulty_scalar(&s.challenge, &s.solution);
        assert_eq!(d, t.expected_scalar);

        let last_difficulty_that_passes = u32::MAX / d;

        assert_eq!(true, check_difficulty(d, last_difficulty_that_passes));
        assert_eq!(false, check_difficulty(d, last_difficulty_that_passes + 1));

        assert!(verify_p2p(
            t.seed,
            t.seed_top64,
            DIFF,
            t.expected_nonce,
            &s.solution.to_bytes(),
        ));
    }
}
