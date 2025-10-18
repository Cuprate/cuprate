use std::task::{Context, Poll};

use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use tower::Service;

use cuprate_blockchain::BlockchainDatabaseService;
use cuprate_database::ConcreteEnv;
use cuprate_fast_sync::validate_entries;
use cuprate_p2p::block_downloader::{ChainSvcRequest, ChainSvcResponse};
use cuprate_p2p_core::NetworkZone;
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};
use crate::blockchain::ConsensusBlockchainReadHandle;
use crate::blockchain::types::BlockchainReadHandle;

/// That service that allows retrieving the chain state to give to the P2P crates, so we can figure out
/// what blocks we need.
///
/// This has a more minimal interface than [`BlockchainReadRequest`] to make using the p2p crates easier.
#[derive(Clone)]
pub struct ChainService(pub BlockchainReadHandle);

impl<N: NetworkZone> Service<ChainSvcRequest<N>> for ChainService {
    type Response = ChainSvcResponse<N>;
    type Error = tower::BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: ChainSvcRequest<N>) -> Self::Future {
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "other requests should be unreachable"
        )]
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
            ChainSvcRequest::ValidateEntries(entries, start_height) => {
                self.0.0.disarm();
                let mut blockchain_read_handle = self.0.clone();

                async move {
                    let (valid, unknown) =
                        validate_entries(entries, start_height, &mut blockchain_read_handle.0)
                            .await?;


                    Ok(ChainSvcResponse::ValidateEntries { valid, unknown })
                }
                .boxed()
            }
        }
    }
}
