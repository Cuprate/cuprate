#![doc = include_str!("../README.md")]
// `proptest` needs this internally.
#![cfg_attr(any(feature = "proptest"), allow(non_local_definitions))]
// Allow some lints when running in debug mode.
#![cfg_attr(debug_assertions, allow(clippy::todo, clippy::multiple_crate_versions))]

//---------------------------------------------------------------------------------------------------- Public API
// Import private modules, export public types.
//
// Documentation for each module is located in the respective file.

mod block_complete_entry;
mod hard_fork;
mod transaction_verification_data;
mod types;

pub use block_complete_entry::{BlockCompleteEntry, PrunedTxBlobEntry, TransactionBlobs};
pub use hard_fork::{HardFork, HardForkError};
pub use transaction_verification_data::{
    CachedVerificationState, TransactionVerificationData, TxVersion,
};
pub use types::{
    AltBlockInformation, Chain, ChainId, ExtendedBlockHeader, OutputOnChain,
    VerifiedBlockInformation, VerifiedTransactionInformation,
};

//---------------------------------------------------------------------------------------------------- Feature-gated
#[cfg(feature = "blockchain")]
pub mod blockchain;

//---------------------------------------------------------------------------------------------------- Private
