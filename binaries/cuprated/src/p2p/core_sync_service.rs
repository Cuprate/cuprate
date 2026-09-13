use std::{
    future::{ready, Ready},
    task::{Context, Poll},
};

use tower::Service;

use cuprate_consensus::BlockchainContextService;
use cuprate_helper::{cast::usize_to_u64, map::split_u128_into_low_high_bits};
use cuprate_p2p_core::services::{CoreSyncDataRequest, CoreSyncDataResponse};
use cuprate_pruning::PruningSeed;
use cuprate_wire::CoreSyncData;

/// The core sync service.
#[derive(Clone)]
pub(crate) struct CoreSyncService {
    pub context_svc: BlockchainContextService,
    pub pruning_seed: PruningSeed,
}

impl Service<CoreSyncDataRequest> for CoreSyncService {
    type Response = CoreSyncDataResponse;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: CoreSyncDataRequest) -> Self::Future {
        let context = self.context_svc.blockchain_context();

        let (cumulative_difficulty, cumulative_difficulty_top64) =
            split_u128_into_low_high_bits(context.cumulative_difficulty);

        ready(Ok(CoreSyncDataResponse(CoreSyncData {
            cumulative_difficulty,
            cumulative_difficulty_top64,
            current_height: usize_to_u64(context.chain_height),
            pruning_seed: self.pruning_seed.compress(),
            top_id: context.top_hash,
            top_version: context.current_hf.as_u8(),
        })))
    }
}
