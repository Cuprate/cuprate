//! TODO
//!
//! TODO: LGPL-3
//!
//! <https://github.com/monero-project/monero/blob/master/docs/POWER.md>

mod p2p;
mod rpc;

pub use p2p::PowerChallengeP2p;
pub use rpc::PowerChallengeRpc;

use blake2::{
    Blake2bVar,
    digest::{Update, VariableOutput},
};

use equix::Solution;

pub use equix;

pub const POWER_INPUT_THRESHOLD: usize = 8;
pub const POWER_HEIGHT_WINDOW: usize = 2;
pub const POWER_DIFFICULTY: u32 = 20;

/// TODO
pub struct PowerSolution {
    /// TODO
    pub challenge: Vec<u8>,
    /// TODO
    pub solution: Solution,
    /// TODO
    pub nonce: u32,
}

mod sealed {
    pub(crate) trait Sealed {}
    impl Sealed for crate::PowerChallengeRpc {}
    impl Sealed for crate::PowerChallengeP2p {}
}

#[expect(private_bounds, reason = "Sealed trait")]
pub trait PowerChallenge
where
    Self: sealed::Sealed
        + Copy
        + Clone
        + std::fmt::Debug
        + std::hash::Hash
        + PartialEq
        + Eq
        + Ord
        + PartialOrd
        + AsRef<[u8]>,
{
    type ChallengeInput;

    const SIZE: usize;

    fn new(challenge: &[u8]) -> Option<Self>;
    fn new_from_input(input: Self::ChallengeInput) -> Self;
    fn update_nonce(&mut self, nonce: u32);

    fn solve(mut self, difficulty: u32) -> PowerSolution {
        for nonce in 0.. {
            let solutions = {
                let Ok(t) = equix::solve(self.as_ref()) else {
                    continue;
                };

                if t.is_empty() {
                    continue;
                }

                t
            };

            for solution in solutions {
                let scalar = equix_solution_to_difficulty_scalar(&solution);

                if check_power_difficulty(scalar, difficulty) {
                    return PowerSolution {
                        challenge: self.as_ref().to_vec(),
                        solution,
                        nonce,
                    };
                }
            }

            self.update_nonce(nonce);
        }

        unreachable!()
    }

    fn verify(&self, solution: &Solution, difficulty: u32) -> bool {
        if equix::verify(self.as_ref(), solution).is_err() {
            return false;
        }

        let scalar = equix_solution_to_difficulty_scalar(solution);
        check_power_difficulty(scalar, difficulty)
    }
}

/// TODO
pub fn equix_solution_to_difficulty_scalar(solution: &Solution) -> u32 {
    let mut h = Blake2bVar::new(4).unwrap();
    h.update(&solution.to_bytes());
    let mut buf = [0; 4];
    h.finalize_variable(&mut buf).unwrap();
    u32::from_le_bytes(buf)
}

/// Returns [`true`] if `scalar * difficulty <= u32::MAX`.
pub const fn check_power_difficulty(scalar: u32, difficulty: u32) -> bool {
    scalar.checked_mul(difficulty).is_some()
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn solve() {
        fn test(test: Vec<(impl PowerChallenge, &'static str, &'static str, u32, u32)>) {
            for (
                challenge,
                expected_challenge,
                expected_solution,
                expected_nonce,
                expected_scalar,
            ) in test
            {
                let p = challenge.solve(POWER_DIFFICULTY);
                let scalar = equix_solution_to_difficulty_scalar(&p.solution);

                assert_eq!(hex::encode(p.challenge), expected_challenge);
                assert_eq!(hex::encode(p.solution.to_bytes()), expected_solution);
                assert_eq!(p.nonce, expected_nonce);
                assert_eq!(scalar, expected_scalar);
            }
        }

        test(vec![(
            PowerChallengeRpc::new_from_input((
                hex!("a12f6872a2178e5eac25f0eb19cc5b29802d3a53e5eea2004756cbfb0af90590"),
                hex!("32d50ed6f691416afc78cb4102821b6392f49bae9a3c2edc513f42564379e936"),
                0,
            )),
            "a12f6872a2178e5eac25f0eb19cc5b29802d3a53e5eea2004756cbfb0af9059032d50ed6f691416afc78cb4102821b6392f49bae9a3c2edc513f42564379e93604000000",
            "a72f5459f39863e96480319203c307fd",
            5,
            108949947,
        )]);
    }
}
