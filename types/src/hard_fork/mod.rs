//! # Hard-Forks
//!
//! Monero use hard-forks to update it's protocol, this module contains a [`HardFork`] enum which is
//! an identifier for every current hard-fork.
//!
//! This module also contains a [`HFVotes`] struct which keeps track of current blockchain voting, and
//! has a method [`HFVotes::current_fork`] to check if the next hard-fork should be activated.

mod constants;
pub use constants::{BLOCK_TIME_V1, BLOCK_TIME_V2, NUMB_OF_HARD_FORKS};

mod error;
pub use error::HardForkError;

mod info;
pub use info::{HFInfo, HFsInfo};

mod hard_fork;
pub use hard_fork::HardFork;

mod votes;
pub use votes::{votes_needed, HFVotes};
