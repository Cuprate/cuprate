//! Pending Service
//!
//! This module contains a [`Future`] that returns ready when a [`PeakEwmaClient`] is ready.
//!
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use tower::Service;

use monero_p2p::{client::PeakEwmaClient, NetworkZone};

/// A [`Future`] that resolves when the inner service is ready.
pub struct PendingService<N: NetworkZone> {
    svc: Option<PeakEwmaClient<N>>,
}

impl<N: NetworkZone> Future for PendingService<N> {
    type Output = Result<PeakEwmaClient<N>, tower::BoxError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.svc
            .as_mut()
            .unwrap()
            .poll_ready(cx)
            .map_ok(|_| self.svc.take().unwrap())
    }
}
