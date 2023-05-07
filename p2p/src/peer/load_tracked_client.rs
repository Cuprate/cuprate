//! A peer connection service wrapper type to handle load tracking and provide access to the
//! reported protocol version.

use std::sync::atomic::Ordering;
use std::{
    sync::Arc,
    task::{Context, Poll},
};

use cuprate_common::PruningSeed;
use tower::{
    load::{Load, PeakEwma},
    Service,
};

use crate::{
    constants::{EWMA_DECAY_TIME_NANOS, EWMA_DEFAULT_RTT},
    peer::{Client, ConnectionInfo},
};

/// A client service wrapper that keeps track of its load.
///
/// It also keeps track of the peer's reported protocol version.
pub struct LoadTrackedClient {
    /// A service representing a connected peer, wrapped in a load tracker.
    service: PeakEwma<Client>,

    /// The metadata for the connected peer `service`.
    connection_info: Arc<ConnectionInfo>,
}

impl LoadTrackedClient {
    pub fn supports_fluffy_blocks(&self) -> bool {
        self.connection_info.support_flags.supports_fluffy_blocks()
    }

    pub fn current_height(&self) -> u64 {
        self.connection_info.peer_height.load(Ordering::SeqCst)
    }

    pub fn pruning_seed(&self) -> PruningSeed {
        self.connection_info.pruning_seed
    }

    pub fn has_full_block(&self, block_height: u64) -> bool {
        let seed = self.pruning_seed();
        let Some(log_stripes) = seed.get_log_stripes() else {
            return true;
        };
        seed.will_have_complete_block(block_height, self.current_height(), log_stripes)
    }
}

/// Create a new [`LoadTrackedClient`] wrapping the provided `client` service.
impl From<Client> for LoadTrackedClient {
    fn from(client: Client) -> Self {
        let connection_info = client.connection_info.clone();

        let service = PeakEwma::new(
            client,
            EWMA_DEFAULT_RTT,
            EWMA_DECAY_TIME_NANOS,
            tower::load::CompleteOnResponse::default(),
        );

        LoadTrackedClient {
            service,
            connection_info,
        }
    }
}

impl<Request> Service<Request> for LoadTrackedClient
where
    Client: Service<Request>,
{
    type Response = <Client as Service<Request>>::Response;
    type Error = <Client as Service<Request>>::Error;
    type Future = <PeakEwma<Client> as Service<Request>>::Future;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(context)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        self.service.call(request)
    }
}

impl Load for LoadTrackedClient {
    type Metric = <PeakEwma<Client> as Load>::Metric;

    fn load(&self) -> Self::Metric {
        self.service.load()
    }
}
