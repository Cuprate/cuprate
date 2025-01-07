#![doc = include_str!("../README.md")]
// `proptest` needs this internally.
#![cfg_attr(any(feature = "proptest"), allow(non_local_definitions))]
// Allow some lints when running in debug mode.
#![cfg_attr(debug_assertions, allow(clippy::todo, clippy::multiple_crate_versions))]

//---------------------------------------------------------------------------------------------------- Public API
// Import private modules, export public types.
//
// Documentation for each module is located in the respective file.

mod address_type;
mod block_complete_entry;
mod connection_state;
mod hard_fork;
mod transaction_verification_data;
mod types;

pub use address_type::AddressType;
pub use block_complete_entry::{BlockCompleteEntry, PrunedTxBlobEntry, TransactionBlobs};
pub use connection_state::ConnectionState;
pub use hard_fork::{HardFork, HardForkError};
pub use transaction_verification_data::{
    CachedVerificationState, TransactionVerificationData, TxVersion,
};
pub use types::{
    AddAuxPow, AltBlockInformation, AuxPow, Chain, ChainId, ChainInfo, CoinbaseTxSum,
    ExtendedBlockHeader, FeeEstimate, HardForkInfo, MinerData, MinerDataTxBacklogEntry,
    OutputHistogramEntry, OutputHistogramInput, OutputOnChain, TxsInBlock,
    VerifiedBlockInformation, VerifiedTransactionInformation,
};

//---------------------------------------------------------------------------------------------------- Feature-gated
#[cfg(feature = "blockchain")]
pub mod blockchain;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "hex")]
pub mod hex;

//---------------------------------------------------------------------------------------------------- Private
