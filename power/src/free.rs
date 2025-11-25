use blake2::{Blake2b, Digest, digest::consts::U4};
use equix::Solution;

use crate::{
    POWER_CHALLENGE_PERSONALIZATION_STRING, PowerChallenge, PowerChallengeP2p, PowerChallengeRpc,
    PowerSolution,
};

/// Create the difficulty scalar used for [`check_difficulty`].
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

/// Solve a PoWER challenge for RPC.
pub fn solve_rpc(
    tx_prefix_hash: [u8; 32],
    recent_block_hash: [u8; 32],
    nonce: u32,
    difficulty: u32,
) -> PowerSolution {
    PowerChallengeRpc::new_from_input((tx_prefix_hash, recent_block_hash, nonce)).solve(difficulty)
}

/// Solve a PoWER challenge for P2P.
pub fn solve_p2p(
    power_challenge_nonce: u64,
    power_challenge_nonce_top64: u64,
    nonce: u32,
    difficulty: u32,
) -> PowerSolution {
    PowerChallengeP2p::new_from_input((power_challenge_nonce, power_challenge_nonce_top64, nonce))
        .solve(difficulty)
}

/// Verify a PoWER challenge for RPC.
///
/// Returns [`true`] if:
/// - `solution` is well-formed.
/// - `solution` satisfies `challenge`.
/// - `solution` passes `difficulty`.
pub fn verify_rpc(
    tx_prefix_hash: [u8; 32],
    recent_block_hash: [u8; 32],
    nonce: u32,
    solution: &[u8; 16],
    difficulty: u32,
) -> bool {
    let Ok(solution) = Solution::try_from_bytes(solution) else {
        return false;
    };
    PowerChallengeRpc::new_from_input((tx_prefix_hash, recent_block_hash, nonce))
        .verify(&solution, difficulty)
}

/// Verify a PoWER challenge for P2P.
///
/// Returns [`true`] if:
/// - `solution` is well-formed.
/// - `solution` satisfies `challenge`.
/// - `solution` passes `difficulty`.
pub fn verify_p2p(
    power_challenge_nonce: u64,
    power_challenge_nonce_top64: u64,
    nonce: u32,
    solution: &[u8; 16],
    difficulty: u32,
) -> bool {
    let Ok(solution) = Solution::try_from_bytes(solution) else {
        return false;
    };
    PowerChallengeP2p::new_from_input((power_challenge_nonce, power_challenge_nonce_top64, nonce))
        .verify(&solution, difficulty)
}
