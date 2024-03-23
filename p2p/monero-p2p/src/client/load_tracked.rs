use futures::lock::OwnedMutexGuard;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use tower::load::{peak_ewma::Cost, CompleteOnResponse, Load, PeakEwma};

use crate::{
    client::{Client, PeerInformation},
    constants::{DEFAULT_RTT, PEAK_EWMA_DECAY_NS},
    NetworkZone,
};

/// A [`Client`] wrapped in a [`PeakEwma`] to keep track of load.
pub struct PeakEwmaClient<N: NetworkZone> {
    /// The client wrapped in [`PeakEwma`]
    client: PeakEwma<Client<N>>,

    /// The connected peer info.
    pub info: PeerInformation<N::Addr>,
}

impl<N: NetworkZone> PeakEwmaClient<N> {
    pub(crate) fn new(
        client: Client<N>,
        mutex_lock: Arc<std::sync::Mutex<Option<OwnedMutexGuard<()>>>>,
    ) -> Self {
        let info = client.info.clone();

        Self {
            client: PeakEwma::new(
                client,
                DEFAULT_RTT,
                PEAK_EWMA_DECAY_NS,
                CompleteOnResponse::default(),
            ),
            info,
        }
    }
}

impl<N: NetworkZone> Deref for PeakEwmaClient<N> {
    type Target = PeakEwma<Client<N>>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl<N: NetworkZone> DerefMut for PeakEwmaClient<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}
