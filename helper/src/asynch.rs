//! `async` related
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use futures::channel::oneshot::Receiver;
use futures::FutureExt;

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
}
