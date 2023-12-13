//! `async` related

//---------------------------------------------------------------------------------------------------- Use
use futures::channel::oneshot::Receiver;
use futures::FutureExt;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

//---------------------------------------------------------------------------------------------------- InfallibleOneshotReceiver
/// A oneshot receiver channel that doesn't return an Error.
///
/// This requires the sender to always return a response.
pub struct InfallibleOneshotReceiver<T>(Receiver<T>);

impl<T> From<Receiver<T>> for InfallibleOneshotReceiver<T> {
    fn from(value: Receiver<T>) -> Self {
        InfallibleOneshotReceiver(value)
    }
}

impl<T> Future for InfallibleOneshotReceiver<T> {
    type Output = T;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0
            .poll_unpin(ctx)
            .map(|res| res.expect("Oneshot must not be cancelled before response!"))
    }
}

//---------------------------------------------------------------------------------------------------- InstaFuture
/// A future that is ready straight away.
pub struct InstaFuture<T>(Option<T>);

impl<T: Unpin> From<T> for InstaFuture<T> {
    fn from(value: T) -> Self {
        InstaFuture(Some(value))
    }
}

impl<T: Unpin> Future for InstaFuture<T> {
    type Output = T;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(
            self.0
                .take()
                .expect("Can't call future twice after Poll::Ready"),
        )
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    // Assert that basic channel operations work.
    async fn infallible_oneshot_receiver() {
        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = "hello world!".to_string();

        tx.send(msg.clone()).unwrap();

        let oneshot = InfallibleOneshotReceiver::from(rx);
        assert_eq!(oneshot.await, msg);
    }

    #[tokio::test]
    // Assert basic value taking works.
    async fn insta_future() {
        let msg = "hello world!";
        let insta = InstaFuture::from(msg.to_string());
        assert_eq!(insta.await, msg);
    }
}
