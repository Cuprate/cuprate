//! Various types (in)directly used in RPC.
//!
//! These types map very closely to types within `cuprate-rpc-types`,
//! however they use more canonical types when appropriate, for example,
//! instead of `hash: String`, this module's types would use something like
//! `hash: [u8; 32]`.
//!
//! - TODO: finish making fields canonical after <https://github.com/Cuprate/cuprate/pull/355>
//! - TODO: can epee handle `u128`? there are a lot of `(top_64 | low_64)` fields

mod pool_info;
mod pool_info_extent;
mod types;

pub use pool_info::PoolInfo;
pub use pool_info_extent::PoolInfoExtent;
pub use types::{
    AddAuxPow, AuxPow, BlockHeader, BlockOutputIndices, ChainInfo, CoinbaseTxSum, ConnectionInfo,
    FeeEstimate, GetBan, GetMinerDataTxBacklogEntry, GetOutputsOut, HardForkEntry, HardForkInfo,
    HistogramEntry, MinerData, MinerDataTxBacklogEntry, OutKey, OutKeyBin, OutputDistributionData,
    OutputHistogramEntry, OutputHistogramInput, Peer, PoolInfoFull, PoolInfoIncremental,
    PoolTxInfo, PublicNode, SetBan, Span, SpentKeyImageInfo, SyncInfoPeer, TxBacklogEntry, TxInfo,
    TxOutputIndices, TxpoolHisto, TxpoolStats,
};
