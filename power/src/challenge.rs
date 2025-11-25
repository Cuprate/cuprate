use equix::Solution;

use crate::{check_difficulty, create_difficulty_scalar};

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
    ///
    /// # Panics
    /// This will technically panic if `difficulty` is set to an
    /// unrealistically high number which prevents a solution from being found.
    ///
    /// It should not panic in real use-cases.
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
