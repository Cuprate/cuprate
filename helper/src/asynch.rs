//! `async` related
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{channel::oneshot, FutureExt};

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
