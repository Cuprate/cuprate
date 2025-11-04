//! Async Buffer
//!
//! A bounded SPSC, FIFO, async buffer that supports arbitrary weights for values.
//!
//! Weight is used to bound the channel, on creation you specify a max weight and for each value you
//! specify a weight.
use std::{
    cmp::min,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    ready,
    task::AtomicWaker,
    Stream, StreamExt,
};

#[derive(thiserror::Error)]
pub enum BufferError<T> {
    #[error("The buffer did not have enough capacity.")]
    NotEnoughCapacity(T),
    #[error("The other end of the buffer disconnected.")]
    Disconnected(T),
}

/// Initializes a new buffer with the provided capacity.
///
/// The capacity inputted is not the max number of items, it is the max combined weight of all items
/// in the buffer.
///
/// It should be noted that if there are no items in the buffer then a single item of any capacity is accepted.
/// i.e. if the capacity is 5 and there are no items in the buffer then any item even if it's weight is >5 will be
/// accepted.
pub fn new_buffer<T>(max_item_weight: usize) -> (BufferAppender<T>, BufferStream<T>) {
    let (tx, rx) = unbounded();
    let sink_waker = Arc::new(AtomicWaker::new());
    let capacity_atomic = Arc::new(AtomicUsize::new(max_item_weight));

    (
        BufferAppender {
            queue: tx,
            sink_waker: sink_waker.clone(),
            capacity: capacity_atomic.clone(),
            max_item_weight,
        },
        BufferStream {
            queue: rx,
            sink_waker,
            capacity: capacity_atomic,
        },
    )
}

/// The stream side of the buffer.
pub struct BufferStream<T> {
    /// The internal queue of items.
    queue: UnboundedReceiver<(T, usize)>,
    /// The waker for the [`BufferAppender`]
    sink_waker: Arc<AtomicWaker>,
    /// The current capacity of the buffer.
    capacity: Arc<AtomicUsize>,
}

impl<T> Stream for BufferStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some((item, size)) = ready!(self.queue.poll_next_unpin(cx)) else {
            return Poll::Ready(None);
        };

        // add the capacity back to the buffer.
        self.capacity.fetch_add(size, Ordering::AcqRel);
        // wake the sink.
        self.sink_waker.wake();

        Poll::Ready(Some(item))
    }
}

/// The appender/sink side of the buffer.
pub struct BufferAppender<T> {
    /// The internal queue of items.
    queue: UnboundedSender<(T, usize)>,
    /// Our waker.
    sink_waker: Arc<AtomicWaker>,
    /// The current capacity of the buffer.
    capacity: Arc<AtomicUsize>,
    /// The max weight of an item, equal to the total allowed weight of the buffer.
    max_item_weight: usize,
}

impl<T> BufferAppender<T> {
    /// Returns a future that resolves when the channel has enough capacity for
    /// a single message of `size_needed`.
    ///
    /// It should be noted that if there are no items in the buffer then a single item of any capacity is accepted.
    /// i.e. if the capacity is 5 and there are no items in the buffer then any item even if it's weight is >5 will be
    /// accepted.
    pub fn ready(&mut self, size_needed: usize) -> BufferSinkReady<'_, T> {
        let size_needed = min(self.max_item_weight, size_needed);

        BufferSinkReady {
            sink: self,
            size_needed,
        }
    }

    /// Attempts to add an item to the buffer.
    ///
    /// # Errors
    /// Returns an error if there is not enough capacity or the [`BufferStream`] was dropped.
    pub fn try_send(&mut self, item: T, size_needed: usize) -> Result<(), BufferError<T>> {
        let size_needed = min(self.max_item_weight, size_needed);

        if self.capacity.load(Ordering::Acquire) < size_needed {
            return Err(BufferError::NotEnoughCapacity(item));
        }

        let prev_size = self.capacity.fetch_sub(size_needed, Ordering::AcqRel);

        // make sure we haven't wrapped the capacity around.
        assert!(prev_size >= size_needed);

        self.queue
            .unbounded_send((item, size_needed))
            .map_err(|e| BufferError::Disconnected(e.into_inner().0))?;

        Ok(())
    }

    /// Waits for capacity in the buffer and then sends the item.
    pub fn send(&mut self, item: T, size_needed: usize) -> BufferSinkSend<'_, T> {
        BufferSinkSend {
            ready: self.ready(size_needed),
            item: Some(item),
        }
    }
}

/// A [`Future`] for adding an item to the buffer.
#[pin_project::pin_project]
pub struct BufferSinkSend<'a, T> {
    /// A future that resolves when the channel has capacity.
    #[pin]
    ready: BufferSinkReady<'a, T>,
    /// The item to send.
    ///
    /// This is [`take`](Option::take)n and added to the buffer when there is enough capacity.
    item: Option<T>,
}

impl<T> Future for BufferSinkSend<'_, T> {
    type Output = Result<(), BufferError<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        let size_needed = this.ready.size_needed;

        this.ready.as_mut().poll(cx).map(|_| {
            this.ready
                .sink
                .try_send(this.item.take().unwrap(), size_needed)
        })
    }
}

/// A [`Future`] for waiting for capacity in the buffer.
pub struct BufferSinkReady<'a, T> {
    /// The sink side of the buffer.
    sink: &'a mut BufferAppender<T>,
    /// The capacity needed.
    ///
    /// This future will wait forever if this is higher than the total availability of the buffer.
    size_needed: usize,
}

impl<T> Future for BufferSinkReady<'_, T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Check before setting the waker just in case it has capacity now,
        if self.sink.capacity.load(Ordering::Acquire) >= self.size_needed {
            return Poll::Ready(());
        }

        // set the waker
        self.sink.sink_waker.register(cx.waker());

        // check the capacity again to avoid a race condition that would result in lost notifications.
        if self.sink.capacity.load(Ordering::Acquire) >= self.size_needed {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
