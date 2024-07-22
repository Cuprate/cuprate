//! TODO

//---------------------------------------------------------------------------------------------------- Import
use serde::{Deserialize, Serialize};

use cuprate_rpc_types::json::{
    AddAuxPowRequest, BannedRequest, CalcPowRequest, FlushCacheRequest,
    FlushTransactionPoolRequest, GenerateBlocksRequest, GetAlternateChainsRequest, GetBansRequest,
    GetBlockCountRequest, GetBlockHeaderByHashRequest, GetBlockHeaderByHeightRequest,
    GetBlockHeadersRangeRequest, GetBlockRequest, GetCoinbaseTxSumRequest, GetConnectionsRequest,
    GetFeeEstimateRequest, GetInfoRequest, GetLastBlockHeaderRequest, GetMinerDataRequest,
    GetOutputHistogramRequest, GetTransactionPoolBacklogRequest, GetVersionRequest,
    HardForkInfoRequest, OnGetBlockHashRequest, PruneBlockchainRequest, RelayTxRequest,
    SetBansRequest, SubmitBlockRequest, SyncInfoRequest,
};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
#[derive(Deserialize, Serialize)]
#[serde(tag = "method", content = "params")]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum JsonRpcMethod {
    GetBlockCount(GetBlockCountRequest),
    OnGetBlockHash(OnGetBlockHashRequest),
    SubmitBlock(SubmitBlockRequest),
    GenerateBlocks(GenerateBlocksRequest),
    GetLastBlockHeader(GetLastBlockHeaderRequest),
    GetBlockHeaderByHash(GetBlockHeaderByHashRequest),
    GetBlockHeaderByHeight(GetBlockHeaderByHeightRequest),
    GetBlockHeadersRange(GetBlockHeadersRangeRequest),
    GetBlock(GetBlockRequest),
    GetConnections(GetConnectionsRequest),
    GetInfo(GetInfoRequest),
    HardForkInfo(HardForkInfoRequest),
    SetBans(SetBansRequest),
    GetBans(GetBansRequest),
    Banned(BannedRequest),
    FlushTransactionPool(FlushTransactionPoolRequest),
    GetOutputHistogram(GetOutputHistogramRequest),
    GetCoinbaseTxSum(GetCoinbaseTxSumRequest),
    GetVersion(GetVersionRequest),
    GetFeeEstimate(GetFeeEstimateRequest),
    GetAlternateChains(GetAlternateChainsRequest),
    RelayTx(RelayTxRequest),
    SyncInfo(SyncInfoRequest),
    GetTransactionPoolBacklog(GetTransactionPoolBacklogRequest),
    GetMinerData(GetMinerDataRequest),
    PruneBlockchain(PruneBlockchainRequest),
    CalcPow(CalcPowRequest),
    FlushCache(FlushCacheRequest),
    AddAuxPow(AddAuxPowRequest),
}

impl JsonRpcMethod {
    /// Returns `true` if this method should
    /// only be allowed on local servers.
    ///
    /// If this returns `false`, it should be
    /// okay to execute the method even on restricted
    /// RPC servers.
    ///
    /// ```rust
    /// use cuprate_rpc_interface::JsonRpcMethod;
    ///
    /// // Allowed method, even on restricted RPC servers (18089).
    /// assert_eq!(JsonRpcMethod::GetBlockCount(()).is_restricted(), false);
    ///
    /// // Restricted methods, only allowed
    /// // for unrestricted RPC servers (18081).
    /// assert_eq!(JsonRpcMethod::GetConnections(()).is_restricted(), true);
    /// ```
    pub const fn is_restricted(&self) -> bool {
        match self {
            // Normal methods. These are allowed
            // even on restricted RPC servers (18089).
            Self::GetBlockCount(())
            | Self::OnGetBlockHash(_)
            | Self::SubmitBlock(_)
            | Self::GetLastBlockHeader(_)
            | Self::GetBlockHeaderByHash(_)
            | Self::GetBlockHeaderByHeight(_)
            | Self::GetBlockHeadersRange(_)
            | Self::GetBlock(_)
            | Self::GetInfo(())
            | Self::HardForkInfo(())
            | Self::GetOutputHistogram(_)
            | Self::GetVersion(())
            | Self::GetFeeEstimate(())
            | Self::GetTransactionPoolBacklog(())
            | Self::GetMinerData(())
            | Self::AddAuxPow(_) => false,

            // Restricted methods. These are only allowed
            // for unrestricted RPC servers (18081).
            Self::GenerateBlocks(_)
            | Self::GetConnections(())
            | Self::SetBans(_)
            | Self::GetBans(())
            | Self::Banned(_)
            | Self::FlushTransactionPool(_)
            | Self::GetCoinbaseTxSum(_)
            | Self::GetAlternateChains(())
            | Self::RelayTx(_)
            | Self::SyncInfo(())
            | Self::PruneBlockchain(_)
            | Self::CalcPow(_)
            | Self::FlushCache(_) => true,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
