use cuprate_blockchain::cuprate_database::RuntimeError;
use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockChainContextService,
};
use cuprate_p2p_core::services::{CoreSyncDataRequest, CoreSyncDataResponse};
use cuprate_p2p_core::CoreSyncData;
use cuprate_types::blockchain::BlockchainReadRequest;
use futures::future::{BoxFuture, MapErr, MapOk};
use futures::{FutureExt, TryFutureExt};
use std::task::{Context, Poll};
use tower::Service;

#[derive(Clone)]
pub struct CoreSyncService(pub BlockChainContextService);

impl Service<CoreSyncDataRequest> for CoreSyncService {
    type Response = CoreSyncDataResponse;
    type Error = tower::BoxError;
    type Future = MapOk<
        <BlockChainContextService as Service<BlockChainContextRequest>>::Future,
        fn(BlockChainContextResponse) -> CoreSyncDataResponse,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, _: CoreSyncDataRequest) -> Self::Future {
        self.0
            .call(BlockChainContextRequest::GetContext)
            .map_ok(|res| {
                let BlockChainContextResponse::Context(ctx) = res else {
                    panic!("blockchain context service returned wrong response.");
                };

                let raw_ctx = ctx.unchecked_blockchain_context();

                // TODO: the hardfork here should be the version of the top block not the current HF,
                // on HF boundaries these will be different.
                CoreSyncDataResponse(CoreSyncData::new(
                    raw_ctx.cumulative_difficulty,
                    // TODO:
                    raw_ctx.chain_height as u64,
                    0,
                    raw_ctx.top_hash,
                    raw_ctx.current_hf.as_u8(),
                ))
            })
    }
}
