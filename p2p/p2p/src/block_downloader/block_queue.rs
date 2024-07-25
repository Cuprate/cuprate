use std::{cmp::Ordering, collections::BinaryHeap};

use cuprate_async_buffer::BufferAppender;

use super::{BlockBatch, BlockDownloadError};

/// A batch of blocks in the ready queue, waiting for previous blocks to come in, so they can
/// be passed into the buffer.
///
/// The [`Eq`] and [`Ord`] impl on this type will only take into account the `start_height`, this
/// is because the block downloader will only download one chain at once so no 2 batches can have
/// the same `start_height`.
///
/// Also, the [`Ord`] impl is reversed so older blocks (lower height) come first in a [`BinaryHeap`].
#[derive(Debug, Clone)]
pub struct ReadyQueueBatch {
    /// The start height of the batch.
    pub start_height: u64,
    /// The batch of blocks.
    pub block_batch: BlockBatch,
}

impl Eq for ReadyQueueBatch {}

impl PartialEq<Self> for ReadyQueueBatch {
    fn eq(&self, other: &Self) -> bool {
        self.start_height.eq(&other.start_height)
    }
}

impl PartialOrd<Self> for ReadyQueueBatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ReadyQueueBatch {
    fn cmp(&self, other: &Self) -> Ordering {
        // reverse the ordering so older blocks (lower height) come first in a [`BinaryHeap`]
        self.start_height.cmp(&other.start_height).reverse()
    }
}

/// The block queue that holds downloaded block batches, adding them to the [`async_buffer`] when the
/// oldest batch has been downloaded.
pub struct BlockQueue {
    /// A queue of ready batches.
    ready_batches: BinaryHeap<ReadyQueueBatch>,
    /// The size, in bytes, of all the batches in [`Self::ready_batches`].
    ready_batches_size: usize,

    /// The [`BufferAppender`] that gives blocks to Cuprate.
    buffer_appender: BufferAppender<BlockBatch>,
}

impl BlockQueue {
    /// Creates a new [`BlockQueue`].
    pub fn new(buffer_appender: BufferAppender<BlockBatch>) -> BlockQueue {
        BlockQueue {
            ready_batches: BinaryHeap::new(),
            ready_batches_size: 0,
            buffer_appender,
        }
    }

    /// Returns the oldest batch that has not been put in the [`async_buffer`] yet.
    pub fn oldest_ready_batch(&self) -> Option<u64> {
        self.ready_batches.peek().map(|batch| batch.start_height)
    }

    /// Returns the size of all the batches that have not been put into the [`async_buffer`] yet.
    pub fn size(&self) -> usize {
        self.ready_batches_size
    }

    /// Adds an incoming batch to the queue and checks if we can push any batches into the [`async_buffer`].
    ///
    /// `oldest_in_flight_start_height` should be the start height of the oldest batch that is still inflight, if
    /// there are no batches inflight then this should be [`None`].
    pub async fn add_incoming_batch(
        &mut self,
        new_batch: ReadyQueueBatch,
        oldest_in_flight_start_height: Option<u64>,
    ) -> Result<(), BlockDownloadError> {
        self.ready_batches_size += new_batch.block_batch.size;
        self.ready_batches.push(new_batch);

        // The height to stop pushing batches into the buffer.
        let height_to_stop_at = oldest_in_flight_start_height.unwrap_or(u64::MAX);

        while self
            .ready_batches
            .peek()
            .is_some_and(|batch| batch.start_height <= height_to_stop_at)
        {
            let batch = self
                .ready_batches
                .pop()
                .expect("We just checked we have a batch in the buffer");

            let batch_size = batch.block_batch.size;

            self.ready_batches_size -= batch_size;
            self.buffer_appender
                .send(batch.block_batch, batch_size)
                .await
                .map_err(|_| BlockDownloadError::BufferWasClosed)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use futures::StreamExt;
    use proptest::{collection::vec, prelude::*};
    use tokio_test::block_on;

    use cuprate_p2p_core::handles::HandleBuilder;

    use super::*;

    prop_compose! {
        fn ready_batch_strategy()(start_height in 0_u64..500_000_000) -> ReadyQueueBatch {
            let (_, peer_handle)  = HandleBuilder::new().build();

            ReadyQueueBatch {
                start_height,
                block_batch: BlockBatch {
                    blocks: vec![],
                    size: start_height as usize,
                    peer_handle,
                },
            }
        }
    }

    proptest! {
        #[test]
        #[allow(clippy::mutable_key_type)]
        fn block_queue_returns_items_in_order(batches in vec(ready_batch_strategy(), 0..10_000)) {
            block_on(async move {
                let (buffer_tx, mut buffer_rx) = cuprate_async_buffer::new_buffer(usize::MAX);

                let mut queue = BlockQueue::new(buffer_tx);

                let mut sorted_batches = BTreeSet::from_iter(batches.clone());
                let mut soreted_batch_2 = sorted_batches.clone();

                for batch in batches {
                    if sorted_batches.remove(&batch) {
                        queue.add_incoming_batch(batch, sorted_batches.last().map(|batch| batch.start_height)).await.unwrap();
                    }
                }

                assert_eq!(queue.size(), 0);
                assert!(queue.oldest_ready_batch().is_none());
                drop(queue);

                while let Some(batch) = buffer_rx.next().await {
                    let last_batch = soreted_batch_2.pop_last().unwrap();

                    assert_eq!(batch.size, last_batch.block_batch.size);
                }
            });
        }
    }
}
