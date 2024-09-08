#![doc = include_str!("../README.md")]
// `proptest` needs this internally.
#![cfg_attr(any(feature = "proptest"), allow(non_local_definitions))]
// Allow some lints when running in debug mode.
#![cfg_attr(debug_assertions, allow(clippy::todo, clippy::multiple_crate_versions))]
#![allow(
    unused_imports,
    unreachable_pub,
    unused_crate_dependencies,
    dead_code,
    unused_variables,
    clippy::needless_pass_by_value,
    clippy::unused_async,
    unreachable_code,
    reason = "TODO: remove after cuprated RpcHandler impl"
)]

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
