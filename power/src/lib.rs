//! # `cuprate-power`
//!
//! This crate contains functionality for [PoWER](https://github.com/monero-project/monero/blob/master/docs/POWER.md).
//!
//! # Solutions for wallets/clients
//!
//! Example of wallet/client logic when relaying transactions.
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
//! let solution: PowerSolution = solve_rpc(
//!     tx_prefix_hash,
//!     recent_block_hash,
//!     nonce,
//!     POWER_DIFFICULTY
//! );
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

#[cfg(test)]
use hex_literal as _;

#[cfg(test)]
mod tests;

mod challenge;
mod constants;
mod free;
mod p2p;
mod rpc;

pub use equix;

pub use challenge::{PowerChallenge, PowerSolution};
pub use constants::{
    POWER_CHALLENGE_PERSONALIZATION_STRING, POWER_DIFFICULTY, POWER_HEIGHT_WINDOW,
    POWER_INPUT_THRESHOLD,
};
pub use free::{
    check_difficulty, create_difficulty_scalar, solve_p2p, solve_rpc, verify_p2p, verify_rpc,
};
pub use p2p::PowerChallengeP2p;
pub use rpc::PowerChallengeRpc;
