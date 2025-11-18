//! # `cuprate-power`
//!
//! This crate contains functionality for [PoWER](https://github.com/monero-project/monero/blob/master/docs/POWER.md).
//!
//! # Solutions for wallets/clients
//! Example of logic for wallets/clients when relaying transactions.
//!
//! ```
//! use cuprate_power::*;
//! use hex_literal::hex;
//!
//! // If transaction inputs <= `POWER_INPUT_THRESHOLD`
//! // then this can be skipped.
//!
//! let tx_prefix_hash = hex!("a12f6872a2178e5eac25f0eb19cc5b29802d3a53e5eea2004756cbfb0af90590");
//! let recent_block_hash = hex!("32d50ed6f691416afc78cb4102821b6392f49bae9a3c2edc513f42564379e936");
//! let nonce = 0;
//!
//! let challenge =PowerChallengeRpc::new_from_input((
//!     tx_prefix_hash,
//!     recent_block_hash,
//!     nonce
//! ));
//!
//! let solution = challenge.solve();
//!
//! // Now include:
//! //
//! // - `tx_prefix_hash`
//! // - `recent_block_hash`
//! // - `solution.solution`
//! // - `solution.nonce`
//! //
//! // when sending a transaction via daemon RPC.
//! ```
//!
//! TODO: LGPL-3
//!
//! <https://github.com/monero-project/monero/blob/master/docs/POWER.md>

mod p2p;
mod rpc;

pub use p2p::PowerChallengeP2p;
pub use rpc::PowerChallengeRpc;

use blake2::{Blake2b, Digest, digest::consts::U4};

use equix::Solution;

pub use equix;

/// Input counts greater than this require PoWER.
pub const POWER_INPUT_THRESHOLD: usize = 8;
/// Number of recent block hashes viable for RPC.
pub const POWER_HEIGHT_WINDOW: usize = 2;
/// Fixed difficulty for valid PoW.
pub const POWER_DIFFICULTY: u32 = 20;
/// Personalization string used in PoWER hashes.
pub const POWER_CHALLENGE_PERSONALIZATION_STRING: &str = "Monero PoWER";

/// Solution to a [`PowerChallenge`].
///
/// This struct contains a valid Equi-X challenge and solution that surpasses a difficulty.
pub struct PowerSolution {
    /// Equi-X challenge bytes.
    pub challenge: Vec<u8>,
    /// Equi-X solution.
    pub solution: Solution,
    /// Nonce input that leads to a valid challenge/solution.
    pub nonce: u32,
}

mod sealed {
    pub(crate) trait Sealed {}
    impl Sealed for crate::PowerChallengeRpc {}
    impl Sealed for crate::PowerChallengeP2p {}
}

#[expect(private_bounds, reason = "Sealed trait")]
/// An Equi-X challenge that must pass a difficulty.
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
    /// Typed Equi-X challenge input.
    type ChallengeInput;

    /// Byte length of [`Self::ChallengeInput`].
    const SIZE: usize;

    /// Create a new [`PowerChallenge`] using raw challenge bytes (including the nonce).
    ///
    /// # Errors
    /// Returns [`None`] if `challenge` are bytes are malformed.
    fn new(challenge: &[u8]) -> Option<Self>;

    /// Create a new [`PowerChallenge`] using a typed challenge.
    fn new_from_input(input: Self::ChallengeInput) -> Self;

    /// Return the current `nonce` used by the challenge.
    fn nonce(&self) -> u32;

    /// Update the `nonce` used by the challenge.
    fn update_nonce(&mut self, nonce: u32);

    /// Attempt to solve this [`PowerChallenge`] using the
    /// current [`PowerChallenge::nonce`] with the given `difficulty`.
    ///
    /// # Errors
    /// Returns [`None`] if no valid solution was found.
    fn try_solve(&self, difficulty: u32) -> Option<PowerSolution> {
        let nonce = self.nonce();

        let solutions = {
            let Ok(t) = equix::solve(self.as_ref()) else {
                return None;
            };

            if t.is_empty() {
                return None;
            }

            t
        };

        for solution in solutions {
            let scalar = create_difficulty_scalar(self.as_ref(), &solution);

            if check_difficulty(scalar, difficulty) {
                return Some(PowerSolution {
                    challenge: self.as_ref().to_vec(),
                    solution,
                    nonce,
                });
            }
        }

        None
    }

    /// Loop through `nonce` values until a solution is found.
    ///
    /// This iterates on `1..` assuming that the
    /// [`PowerSolution::nonce`] is set to `0`.
    fn solve(mut self, difficulty: u32) -> PowerSolution {
        for nonce in 1.. {
            if let Some(t) = self.try_solve(difficulty) {
                return t;
            }

            self.update_nonce(nonce);
        }

        unreachable!()
    }

    /// Verify that `solution`:
    /// - is a valid Equi-X solution for this [`PowerChallenge`].
    /// - satisfies `difficulty`.
    fn verify(&self, solution: &Solution, difficulty: u32) -> bool {
        if equix::verify(self.as_ref(), solution).is_err() {
            return false;
        }

        let scalar = create_difficulty_scalar(self.as_ref(), solution);
        check_difficulty(scalar, difficulty)
    }
}

/// Create the diffculty scalar used for [`check_difficulty`].
pub fn create_difficulty_scalar(challenge: &[u8], solution: &Solution) -> u32 {
    let mut h = Blake2b::<U4>::new();
    h.update(POWER_CHALLENGE_PERSONALIZATION_STRING.as_bytes());
    h.update(challenge);
    h.update(solution.to_bytes());
    u32::from_le_bytes(h.finalize().into())
}

/// Returns [`true`] if `scalar * difficulty <= u32::MAX`.
pub const fn check_difficulty(scalar: u32, difficulty: u32) -> bool {
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
            let difficulty = POWER_DIFFICULTY;

            for (
                challenge,
                expected_challenge,
                expected_solution,
                expected_nonce,
                expected_scalar,
            ) in test
            {
                let p = challenge.solve(difficulty);
                let scalar = create_difficulty_scalar(&p.challenge, &p.solution);

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
