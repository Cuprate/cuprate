//! `async` related
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
use core::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use std::sync::Arc;

use futures::{channel::oneshot, ready, FutureExt};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::PollSemaphore;

//---------------------------------------------------------------------------------------------------- InfallibleOneshotReceiver
/// A oneshot receiver channel that doesn't return an Error.
///
/// This requires the sender to always return a response.
pub struct InfallibleOneshotReceiver<T>(oneshot::Receiver<T>);

impl<T> From<oneshot::Receiver<T>> for InfallibleOneshotReceiver<T> {
    fn from(value: oneshot::Receiver<T>) -> Self {
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

//---------------------------------------------------------------------------------------------------- rayon_spawn_async
/// Spawns a task for the rayon thread pool and awaits the result without blocking the async runtime.
pub async fn rayon_spawn_async<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    rayon::spawn(move || {
        let _ = tx.send(f());
    });
    rx.await.expect("The sender must not be dropped")
}

//---------------------------------------------------------------------------------------------------- Poll Sender
mod hidden {

    pub trait UnboundedChannel<T>: Clone {
        fn send(&mut self, message: T);
    }

    impl<T> UnboundedChannel<T> for tokio::sync::mpsc::UnboundedSender<T> {
        fn send(&mut self, message: T) {
            tokio::sync::mpsc::UnboundedSender::send(self, message)
                .expect("Unable to send message receivers dropped!")
        }
    }
}

/// Create a new channel with the sender wrapped in [`PollSender`].
///
/// ```rust
/// use tokio::sync::mpsc;
///
/// use cuprate_helper::asynch::poll_sender_channel;
/// // although the inner channel is unbounded the `PollSender` will apply a limit.
/// let (tx, rx) = poll_sender_channel::<_, _, u8>(mpsc::unbounded_channel, 3);
///
/// ```
pub fn poll_sender_channel<C: hidden::UnboundedChannel<T>, R, T>(
    new_inner: impl FnOnce() -> (C, R),
    buffer: usize,
) -> (PollSender<C, T>, R) {
    let (inner_tx, inner_rx) = new_inner();

    let semaphore = Arc::new(Semaphore::new(buffer));
    let poll_semaphore = PollSemaphore::new(semaphore);

    (
        PollSender {
            channel: inner_tx,
            semaphore: poll_semaphore,
            permit: None,
            ty: PhantomData,
        },
        inner_rx,
    )
}

/// The channel was closed.
#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("The channel was closed.")]
pub struct ChannelClosedError;

/// This is a wrapper around a channel's send half, that adds a method [`PollSender::poll_ready`] to asses is the channel
/// has enough capacity to receive a message.
#[derive(Debug)]
pub struct PollSender<C: hidden::UnboundedChannel<T>, T> {
    channel: C,
    semaphore: PollSemaphore,
    permit: Option<OwnedSemaphorePermit>,

    ty: PhantomData<T>,
}

impl<C: hidden::UnboundedChannel<T>, T> Clone for PollSender<C, T> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
            semaphore: self.semaphore.clone(),
            permit: None,
            ty: PhantomData,
        }
    }
}

impl<C: hidden::UnboundedChannel<T>, T> PollSender<C, T> {
    /// Polls the channel to check if it has enough capacity to receive a message. When this function returns [`Poll::Ready`]
    /// a spot in the channel has been reserved for a message, this means the channel will lose a spot until [`PollSender::send`]
    /// is called or the [`PollSender`] is dropped.
    pub fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ChannelClosedError>> {
        if self.permit.is_some() {
            return Poll::Ready(Ok(()));
        }

        let Some(permit) = ready!(self.semaphore.poll_acquire(cx)) else {
            return Poll::Ready(Err(ChannelClosedError));
        };
        self.permit = Some(permit);

        Poll::Ready(Ok(()))
    }

    /// Sends a message on the channel, will panic if [`PollSender::poll_ready`] has not returned [`Poll::Ready`].
    pub fn send(&mut self, mes: T) {
        let _permit = self
            .permit
            .take()
            .expect("`poll_ready must be called first!`");

        self.channel.send(mes)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use std::{
        sync::{Arc, Barrier},
        thread,
        time::Duration,
    };

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

    #[test]
    fn rayon_spawn_async_does_not_block() {
        // There must be more than 1 rayon thread for this to work.
        rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build_global()
            .unwrap();

        // We use a barrier to make sure both tasks are executed together, we block the rayon thread
        // until both rayon threads are blocked.
        let barrier = Arc::new(Barrier::new(2));
        let task = |barrier: &Barrier| barrier.wait();

        let b_2 = barrier.clone();

        let (tx, rx) = std::sync::mpsc::channel();

        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                tokio::join!(
                    // This polls them concurrently in the same task, so if the first one blocks the task then
                    // the second wont run and if the second does not run the first does not unblock.
                    rayon_spawn_async(move || task(&barrier)),
                    rayon_spawn_async(move || task(&b_2)),
                )
            });

            // if we managed to get here then rayon_spawn_async didn't block.
            tx.send(()).unwrap();
        });

        rx.recv_timeout(Duration::from_secs(2))
            .expect("rayon_spawn_async blocked");
    }
}
