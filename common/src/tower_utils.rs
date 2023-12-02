use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::channel::oneshot;
use futures::FutureExt;

/// A oneshot that doesn't return an Error. This requires the sender to always
/// return a response.
pub struct InfallibleOneshotReceiver<T>(oneshot::Receiver<T>);

impl<T> From<oneshot::Receiver<T>> for InfallibleOneshotReceiver<T> {
    fn from(value: oneshot::Receiver<T>) -> Self {
        InfallibleOneshotReceiver(value)
    }
}

impl<T> Future for InfallibleOneshotReceiver<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0
            .poll_unpin(cx)
            .map(|res| res.expect("Oneshot must not be cancelled before response!"))
    }
}

/// A future that is ready straight away.
pub struct InstaFuture<T>(Option<T>);

impl<T: Unpin> From<T> for InstaFuture<T> {
    fn from(value: T) -> Self {
        InstaFuture(Some(value))
    }
}

impl<T: Unpin> Future for InstaFuture<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(
            self.0
                .take()
                .expect("Can't call future twice after Poll::Ready"),
        )
    }
}
