use std::task::{Context, Poll};

use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use tower::Service;

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_p2p::block_downloader::{ChainSvcRequest, ChainSvcResponse};
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};

/// That service that allows retrieving the chain state to give to the P2P crates, so we can figure out
/// what blocks we need.
///
/// This has a more minimal interface than [`BlockchainReadRequest`] to make using the p2p crates easier.
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
            _ => unreachable!(),
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
                    // TODO: use the context cache.
                    let BlockchainResponse::CompactChainHistory {
                        cumulative_difficulty,
                        ..
                    } = res
                    else {
                        unreachable!()
                    };

                    ChainSvcResponse::CumulativeDifficulty(cumulative_difficulty)
                })
                .map_err(Into::into)
                .boxed(),
        }
    }
}
