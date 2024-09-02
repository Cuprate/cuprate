//! Dummy implementation of [`RpcHandler`].

//---------------------------------------------------------------------------------------------------- Use
use std::task::Poll;

use futures::channel::oneshot::channel;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{
    rpc_error::RpcError, rpc_handler::RpcHandler, rpc_request::RpcRequest,
    rpc_response::RpcResponse,
};

//---------------------------------------------------------------------------------------------------- RpcHandlerDummy
/// An [`RpcHandler`] that always returns [`Default::default`].
///
/// This `struct` implements [`RpcHandler`], and always responds
/// with the response `struct` set to [`Default::default`].
///
/// See the [`crate`] documentation for example usage.
///
/// This is mostly used for testing purposes and can
/// be disabled by disable the `dummy` feature flag.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct RpcHandlerDummy {
    /// Should this RPC server be [restricted](RpcHandler::restricted)?
    ///
    /// The dummy will honor this [`bool`]
    /// on restricted methods/endpoints.
    pub restricted: bool,
}

impl RpcHandler for RpcHandlerDummy {
    fn restricted(&self) -> bool {
        self.restricted
    }
}

impl Service<RpcRequest> for RpcHandlerDummy {
    type Response = RpcResponse;
    type Error = RpcError;
    type Future = InfallibleOneshotReceiver<Result<RpcResponse, RpcError>>;

    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RpcRequest) -> Self::Future {
        use cuprate_rpc_types::bin::BinRequest as BReq;
        use cuprate_rpc_types::bin::BinResponse as BResp;
        use cuprate_rpc_types::json::JsonRpcRequest as JReq;
        use cuprate_rpc_types::json::JsonRpcResponse as JResp;
        use cuprate_rpc_types::other::OtherRequest as OReq;
        use cuprate_rpc_types::other::OtherResponse as OResp;

        #[rustfmt::skip]
        #[allow(clippy::default_trait_access)]
        let resp = match req {
            RpcRequest::JsonRpc(j) => RpcResponse::JsonRpc(match j {
                JReq::GetBlockCount(_) => JResp::GetBlockCount(Default::default()),
                JReq::OnGetBlockHash(_) => JResp::OnGetBlockHash(Default::default()),
                JReq::SubmitBlock(_) => JResp::SubmitBlock(Default::default()),
                JReq::GenerateBlocks(_) => JResp::GenerateBlocks(Default::default()),
                JReq::GetLastBlockHeader(_) => JResp::GetLastBlockHeader(Default::default()),
                JReq::GetBlockHeaderByHash(_) => JResp::GetBlockHeaderByHash(Default::default()),
                JReq::GetBlockHeaderByHeight(_) => JResp::GetBlockHeaderByHeight(Default::default()),
                JReq::GetBlockHeadersRange(_) => JResp::GetBlockHeadersRange(Default::default()),
                JReq::GetBlock(_) => JResp::GetBlock(Default::default()),
                JReq::GetConnections(_) => JResp::GetConnections(Default::default()),
                JReq::GetInfo(_) => JResp::GetInfo(Default::default()),
                JReq::HardForkInfo(_) => JResp::HardForkInfo(Default::default()),
                JReq::SetBans(_) => JResp::SetBans(Default::default()),
                JReq::GetBans(_) => JResp::GetBans(Default::default()),
                JReq::Banned(_) => JResp::Banned(Default::default()),
                JReq::FlushTransactionPool(_) => JResp::FlushTransactionPool(Default::default()),
                JReq::GetOutputHistogram(_) => JResp::GetOutputHistogram(Default::default()),
                JReq::GetCoinbaseTxSum(_) => JResp::GetCoinbaseTxSum(Default::default()),
                JReq::GetVersion(_) => JResp::GetVersion(Default::default()),
                JReq::GetFeeEstimate(_) => JResp::GetFeeEstimate(Default::default()),
                JReq::GetAlternateChains(_) => JResp::GetAlternateChains(Default::default()),
                JReq::RelayTx(_) => JResp::RelayTx(Default::default()),
                JReq::SyncInfo(_) => JResp::SyncInfo(Default::default()),
                JReq::GetTransactionPoolBacklog(_) => JResp::GetTransactionPoolBacklog(Default::default()),
                JReq::GetMinerData(_) => JResp::GetMinerData(Default::default()),
                JReq::PruneBlockchain(_) => JResp::PruneBlockchain(Default::default()),
                JReq::CalcPow(_) => JResp::CalcPow(Default::default()),
                JReq::FlushCache(_) => JResp::FlushCache(Default::default()),
                JReq::AddAuxPow(_) => JResp::AddAuxPow(Default::default()),
                JReq::GetTxIdsLoose(_) => JResp::GetTxIdsLoose(Default::default()),
            }),
            RpcRequest::Binary(b) => RpcResponse::Binary(match b {
                BReq::GetBlocks(_) => BResp::GetBlocks(Default::default()),
                BReq::GetBlocksByHeight(_) => BResp::GetBlocksByHeight(Default::default()),
                BReq::GetHashes(_) => BResp::GetHashes(Default::default()),
                BReq::GetOutputIndexes(_) => BResp::GetOutputIndexes(Default::default()),
                BReq::GetOuts(_) => BResp::GetOuts(Default::default()),
                BReq::GetTransactionPoolHashes(_) => BResp::GetTransactionPoolHashes(Default::default()),
                BReq::GetOutputDistribution(_) => BResp::GetOutputDistribution(Default::default()),
            }),
            RpcRequest::Other(o) => RpcResponse::Other(match o {
                OReq::GetHeight(_) => OResp::GetHeight(Default::default()),
                OReq::GetTransactions(_) => OResp::GetTransactions(Default::default()),
                OReq::GetAltBlocksHashes(_) => OResp::GetAltBlocksHashes(Default::default()),
                OReq::IsKeyImageSpent(_) => OResp::IsKeyImageSpent(Default::default()),
                OReq::SendRawTransaction(_) => OResp::SendRawTransaction(Default::default()),
                OReq::StartMining(_) => OResp::StartMining(Default::default()),
                OReq::StopMining(_) => OResp::StopMining(Default::default()),
                OReq::MiningStatus(_) => OResp::MiningStatus(Default::default()),
                OReq::SaveBc(_) => OResp::SaveBc(Default::default()),
                OReq::GetPeerList(_) => OResp::GetPeerList(Default::default()),
                OReq::SetLogHashRate(_) => OResp::SetLogHashRate(Default::default()),
                OReq::SetLogLevel(_) => OResp::SetLogLevel(Default::default()),
                OReq::SetLogCategories(_) => OResp::SetLogCategories(Default::default()),
                OReq::SetBootstrapDaemon(_) => OResp::SetBootstrapDaemon(Default::default()),
                OReq::GetTransactionPool(_) => OResp::GetTransactionPool(Default::default()),
                OReq::GetTransactionPoolStats(_) => OResp::GetTransactionPoolStats(Default::default()),
                OReq::StopDaemon(_) => OResp::StopDaemon(Default::default()),
                OReq::GetLimit(_) => OResp::GetLimit(Default::default()),
                OReq::SetLimit(_) => OResp::SetLimit(Default::default()),
                OReq::OutPeers(_) => OResp::OutPeers(Default::default()),
                OReq::InPeers(_) => OResp::InPeers(Default::default()),
                OReq::GetNetStats(_) => OResp::GetNetStats(Default::default()),
                OReq::GetOuts(_) => OResp::GetOuts(Default::default()),
                OReq::Update(_) => OResp::Update(Default::default()),
                OReq::PopBlocks(_) => OResp::PopBlocks(Default::default()),
                OReq::GetTransactionPoolHashes(_) => OResp::GetTransactionPoolHashes(Default::default()),
                OReq::GetPublicNodes(_) => OResp::GetPublicNodes(Default::default()),
            })
        };

        let (tx, rx) = channel();
        drop(tx.send(Ok(resp)));
        InfallibleOneshotReceiver::from(rx)
    }
}
