use std::task::{Context, Poll};

use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use tower::Service;

use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockChainContextService,
};
use cuprate_helper::{cast::usize_to_u64, map::split_u128_into_low_high_bits};
use cuprate_p2p_core::services::{CoreSyncDataRequest, CoreSyncDataResponse};
use cuprate_wire::CoreSyncData;

/// The core sync service.
#[derive(Clone)]
pub struct CoreSyncService(pub BlockChainContextService);

impl Service<CoreSyncDataRequest> for CoreSyncService {
    type Response = CoreSyncDataResponse;
    type Error = tower::BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, _: CoreSyncDataRequest) -> Self::Future {
        self.0
            .call(BlockChainContextRequest::Context)
            .map_ok(|res| {
                let BlockChainContextResponse::Context(context) = res else {
                    unreachable!()
                };

                let context = context.unchecked_blockchain_context();
                let (cumulative_difficulty, cumulative_difficulty_top64) =
                    split_u128_into_low_high_bits(context.cumulative_difficulty);

                CoreSyncDataResponse(CoreSyncData {
                    cumulative_difficulty,
                    cumulative_difficulty_top64,
                    current_height: usize_to_u64(context.chain_height),
                    pruning_seed: 0,
                    top_id: context.top_hash,
                    top_version: context.current_hf.as_u8(),
                })
            })
            .boxed()
    }
}
