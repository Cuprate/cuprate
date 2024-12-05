//! Miscellaneous types.
//!
//! These are data types that appear in request/response types.
//!
//! For example, [`crate::json::GetConnectionsResponse`] contains
//! the [`crate::misc::ConnectionInfo`] struct defined here.

//---------------------------------------------------------------------------------------------------- Lints
#![allow(
    missing_docs, // Docs are at: <https://www.getmonero.org/resources/developer-guides/daemon-rpc.html>
    clippy::struct_excessive_bools, // hey man, tell that to the people who wrote `monerod`
)]

//---------------------------------------------------------------------------------------------------- Mod
mod binary_string;
mod distribution;
mod key_image_spent_status;
mod requested_info;
mod status;
mod tx_entry;
mod types;

pub use binary_string::BinaryString;
pub use distribution::{Distribution, DistributionCompressedBinary, DistributionUncompressed};
pub use key_image_spent_status::KeyImageSpentStatus;
pub use requested_info::RequestedInfo;
pub use status::Status;
pub use tx_entry::TxEntry;
pub use types::{
    AuxPow, BlockHeader, BlockOutputIndices, ChainInfo, ConnectionInfo, GetBan,
    GetMinerDataTxBacklogEntry, GetOutputsOut, HardforkEntry, HistogramEntry, OutKey, OutKeyBin,
    OutputDistributionData, Peer, PublicNode, SetBan, Span, SpentKeyImageInfo, SyncInfoPeer,
    TxBacklogEntry, TxInfo, TxOutputIndices, TxpoolHisto, TxpoolStats,
};
