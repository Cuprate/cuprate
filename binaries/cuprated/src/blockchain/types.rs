use cuprate_blockchain::cuprate_database::RuntimeError;
use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::{BlockChainContextService, BlockVerifierService, TxVerifierService};
use cuprate_p2p::block_downloader::{ChainSvcRequest, ChainSvcResponse};
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};
use futures::future::{BoxFuture, MapErr};
use futures::{FutureExt, TryFutureExt};
use std::task::{Context, Poll};
use tower::Service;

pub type ConcreteBlockVerifierService = BlockVerifierService<
    BlockChainContextService,
    TxVerifierService<ConsensusBlockchainReadHandle>,
    ConsensusBlockchainReadHandle,
>;

pub type ConcreteTxVerifierService = TxVerifierService<ConsensusBlockchainReadHandle>;

#[derive(Clone)]
pub struct ConsensusBlockchainReadHandle(pub BlockchainReadHandle);

impl Service<BlockchainReadRequest> for ConsensusBlockchainReadHandle {
    type Response = BlockchainResponse;
    type Error = tower::BoxError;
    type Future = MapErr<
        <BlockchainReadHandle as Service<BlockchainReadRequest>>::Future,
        fn(RuntimeError) -> tower::BoxError,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: BlockchainReadRequest) -> Self::Future {
        self.0.call(req).map_err(Into::into)
    }
}

#[derive(Clone)]
pub struct ChainService(pub BlockchainReadHandle);

impl Service<ChainSvcRequest> for ChainService {
    type Response = ChainSvcResponse;
    type Error = tower::BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: ChainSvcRequest) -> Self::Future {
        let map_res = |res: BlockchainResponse| match res {
            BlockchainResponse::CompactChainHistory {
                block_ids,
                cumulative_difficulty,
            } => ChainSvcResponse::CompactHistory {
                block_ids,
                cumulative_difficulty,
            },
            BlockchainResponse::FindFirstUnknown(res) => ChainSvcResponse::FindFirstUnknown(res),
            _ => panic!("Blockchain returned wrong response"),
        };

        match req {
            ChainSvcRequest::CompactHistory => self
                .0
                .call(BlockchainReadRequest::CompactChainHistory)
                .map_ok(map_res)
                .map_err(Into::into)
                .boxed(),
            ChainSvcRequest::FindFirstUnknown(req) => self
                .0
                .call(BlockchainReadRequest::FindFirstUnknown(req))
                .map_ok(map_res)
                .map_err(Into::into)
                .boxed(),
            ChainSvcRequest::CumulativeDifficulty => self
                .0
                .call(BlockchainReadRequest::CompactChainHistory)
                .map_ok(|res| {
                    // TODO create a custom request instead of hijacking this one.
                    let BlockchainResponse::CompactChainHistory {
                        cumulative_difficulty,
                        ..
                    } = res
                    else {
                        panic!("Blockchain returned wrong response");
                    };

                    ChainSvcResponse::CumulativeDifficulty(cumulative_difficulty)
                })
                .map_err(Into::into)
                .boxed(),
        }
    }
}
