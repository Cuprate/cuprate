//! Dummy implementation of [`RpcHandler`].

//---------------------------------------------------------------------------------------------------- Use
use std::task::Poll;

use anyhow::Error;
use futures::channel::oneshot::channel;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_rpc_types::{
    bin::{BinRequest, BinResponse},
    json::{JsonRpcRequest, JsonRpcResponse},
    other::{OtherRequest, OtherResponse},
};

use crate::rpc_handler::RpcHandler;

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
    fn is_restricted(&self) -> bool {
        self.restricted
    }
}

impl Service<JsonRpcRequest> for RpcHandlerDummy {
    type Response = JsonRpcResponse;
    type Error = Error;
    type Future = InfallibleOneshotReceiver<Result<JsonRpcResponse, Error>>;

    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: JsonRpcRequest) -> Self::Future {
        use cuprate_rpc_types::json::JsonRpcRequest as Req;
        use cuprate_rpc_types::json::JsonRpcResponse as Resp;

        #[expect(clippy::default_trait_access)]
        let resp = match req {
            Req::GetBlockCount(_) => Resp::GetBlockCount(Default::default()),
            Req::OnGetBlockHash(_) => Resp::OnGetBlockHash(Default::default()),
            Req::SubmitBlock(_) => Resp::SubmitBlock(Default::default()),
            Req::GenerateBlocks(_) => Resp::GenerateBlocks(Default::default()),
            Req::GetLastBlockHeader(_) => Resp::GetLastBlockHeader(Default::default()),
            Req::GetBlockHeaderByHash(_) => Resp::GetBlockHeaderByHash(Default::default()),
            Req::GetBlockHeaderByHeight(_) => Resp::GetBlockHeaderByHeight(Default::default()),
            Req::GetBlockHeadersRange(_) => Resp::GetBlockHeadersRange(Default::default()),
            Req::GetBlock(_) => Resp::GetBlock(Default::default()),
            Req::GetConnections(_) => Resp::GetConnections(Default::default()),
            Req::GetInfo(_) => Resp::GetInfo(Default::default()),
            Req::HardForkInfo(_) => Resp::HardForkInfo(Default::default()),
            Req::SetBans(_) => Resp::SetBans(Default::default()),
            Req::GetBans(_) => Resp::GetBans(Default::default()),
            Req::Banned(_) => Resp::Banned(Default::default()),
            Req::FlushTransactionPool(_) => Resp::FlushTransactionPool(Default::default()),
            Req::GetOutputHistogram(_) => Resp::GetOutputHistogram(Default::default()),
            Req::GetCoinbaseTxSum(_) => Resp::GetCoinbaseTxSum(Default::default()),
            Req::GetVersion(_) => Resp::GetVersion(Default::default()),
            Req::GetFeeEstimate(_) => Resp::GetFeeEstimate(Default::default()),
            Req::GetAlternateChains(_) => Resp::GetAlternateChains(Default::default()),
            Req::RelayTx(_) => Resp::RelayTx(Default::default()),
            Req::SyncInfo(_) => Resp::SyncInfo(Default::default()),
            Req::GetTransactionPoolBacklog(_) => {
                Resp::GetTransactionPoolBacklog(Default::default())
            }
            Req::GetMinerData(_) => Resp::GetMinerData(Default::default()),
            Req::PruneBlockchain(_) => Resp::PruneBlockchain(Default::default()),
            Req::CalcPow(_) => Resp::CalcPow(Default::default()),
            Req::FlushCache(_) => Resp::FlushCache(Default::default()),
            Req::AddAuxPow(_) => Resp::AddAuxPow(Default::default()),
            Req::GetTxIdsLoose(_) => Resp::GetTxIdsLoose(Default::default()),
        };

        let (tx, rx) = channel();
        drop(tx.send(Ok(resp)));
        InfallibleOneshotReceiver::from(rx)
    }
}

impl Service<BinRequest> for RpcHandlerDummy {
    type Response = BinResponse;
    type Error = Error;
    type Future = InfallibleOneshotReceiver<Result<BinResponse, Error>>;

    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: BinRequest) -> Self::Future {
        use cuprate_rpc_types::bin::BinRequest as Req;
        use cuprate_rpc_types::bin::BinResponse as Resp;

        #[expect(clippy::default_trait_access)]
        let resp = match req {
            Req::GetBlocks(_) => Resp::GetBlocks(Default::default()),
            Req::GetBlocksByHeight(_) => Resp::GetBlocksByHeight(Default::default()),
            Req::GetHashes(_) => Resp::GetHashes(Default::default()),
            Req::GetOutputIndexes(_) => Resp::GetOutputIndexes(Default::default()),
            Req::GetOuts(_) => Resp::GetOuts(Default::default()),
            Req::GetTransactionPoolHashes(_) => Resp::GetTransactionPoolHashes(Default::default()),
            Req::GetOutputDistribution(_) => Resp::GetOutputDistribution(Default::default()),
        };

        let (tx, rx) = channel();
        drop(tx.send(Ok(resp)));
        InfallibleOneshotReceiver::from(rx)
    }
}

impl Service<OtherRequest> for RpcHandlerDummy {
    type Response = OtherResponse;
    type Error = Error;
    type Future = InfallibleOneshotReceiver<Result<OtherResponse, Error>>;

    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: OtherRequest) -> Self::Future {
        use cuprate_rpc_types::other::OtherRequest as Req;
        use cuprate_rpc_types::other::OtherResponse as Resp;

        #[expect(clippy::default_trait_access)]
        let resp = match req {
            Req::GetHeight(_) => Resp::GetHeight(Default::default()),
            Req::GetTransactions(_) => Resp::GetTransactions(Default::default()),
            Req::GetAltBlocksHashes(_) => Resp::GetAltBlocksHashes(Default::default()),
            Req::IsKeyImageSpent(_) => Resp::IsKeyImageSpent(Default::default()),
            Req::SendRawTransaction(_) => Resp::SendRawTransaction(Default::default()),
            Req::StartMining(_) => Resp::StartMining(Default::default()),
            Req::StopMining(_) => Resp::StopMining(Default::default()),
            Req::MiningStatus(_) => Resp::MiningStatus(Default::default()),
            Req::SaveBc(_) => Resp::SaveBc(Default::default()),
            Req::GetPeerList(_) => Resp::GetPeerList(Default::default()),
            Req::SetLogHashRate(_) => Resp::SetLogHashRate(Default::default()),
            Req::SetLogLevel(_) => Resp::SetLogLevel(Default::default()),
            Req::SetLogCategories(_) => Resp::SetLogCategories(Default::default()),
            Req::SetBootstrapDaemon(_) => Resp::SetBootstrapDaemon(Default::default()),
            Req::GetTransactionPool(_) => Resp::GetTransactionPool(Default::default()),
            Req::GetTransactionPoolStats(_) => Resp::GetTransactionPoolStats(Default::default()),
            Req::StopDaemon(_) => Resp::StopDaemon(Default::default()),
            Req::GetLimit(_) => Resp::GetLimit(Default::default()),
            Req::SetLimit(_) => Resp::SetLimit(Default::default()),
            Req::OutPeers(_) => Resp::OutPeers(Default::default()),
            Req::InPeers(_) => Resp::InPeers(Default::default()),
            Req::GetNetStats(_) => Resp::GetNetStats(Default::default()),
            Req::GetOuts(_) => Resp::GetOuts(Default::default()),
            Req::Update(_) => Resp::Update(Default::default()),
            Req::PopBlocks(_) => Resp::PopBlocks(Default::default()),
            Req::GetTransactionPoolHashes(_) => Resp::GetTransactionPoolHashes(Default::default()),
            Req::GetPublicNodes(_) => Resp::GetPublicNodes(Default::default()),
        };

        let (tx, rx) = channel();
        drop(tx.send(Ok(resp)));
        InfallibleOneshotReceiver::from(rx)
    }
}
