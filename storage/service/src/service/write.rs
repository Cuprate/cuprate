use std::{
    fmt::Debug,
    sync::Arc,
    task::{Context, Poll},
};

use futures::channel::oneshot;
use tracing::{info, warn};

use cuprate_database::{ConcreteEnv, DbResult, Env, RuntimeError};
use cuprate_helper::asynch::InfallibleOneshotReceiver;

//---------------------------------------------------------------------------------------------------- Constants
/// Name of the writer thread.
const WRITER_THREAD_NAME: &str = concat!(module_path!(), "::DatabaseWriter");

//---------------------------------------------------------------------------------------------------- DatabaseWriteHandle
/// Write handle to the database.
///
/// This is handle that allows `async`hronously writing to the database.
///
/// Calling [`tower::Service::call`] with a [`DatabaseWriteHandle`]
/// will return an `async`hronous channel that can be `.await`ed upon
/// to receive the corresponding response.
#[derive(Debug)]
pub struct DatabaseWriteHandle<Req, Res> {
    /// Sender channel to the database write thread-pool.
    ///
    /// We provide the response channel for the thread-pool.
    pub(super) sender: crossbeam::channel::Sender<(Req, oneshot::Sender<DbResult<Res>>)>,
}

impl<Req, Res> Clone for DatabaseWriteHandle<Req, Res> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<Req, Res> DatabaseWriteHandle<Req, Res>
where
    Req: Send + 'static,
    Res: Debug + Send + 'static,
{
    /// Initialize the single `DatabaseWriter` thread.
    #[cold]
    #[inline(never)] // Only called once.
    pub fn init(
        env: Arc<ConcreteEnv>,
        inner_handler: impl Fn(&ConcreteEnv, &Req) -> DbResult<Res> + Send + 'static,
    ) -> Self {
        // Initialize `Request/Response` channels.
        let (sender, receiver) = crossbeam::channel::unbounded();

        // Spawn the writer.
        std::thread::Builder::new()
            .name(WRITER_THREAD_NAME.into())
            .spawn(move || database_writer(&env, &receiver, inner_handler))
            .unwrap();

        Self { sender }
    }
}

impl<Req, Res> tower::Service<Req> for DatabaseWriteHandle<Req, Res> {
    type Response = Res;
    type Error = RuntimeError;
    type Future = InfallibleOneshotReceiver<DbResult<Res>>;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<DbResult<()>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, request: Req) -> Self::Future {
        // Response channel we `.await` on.
        let (response_sender, receiver) = oneshot::channel();

        // Send the write request.
        self.sender.send((request, response_sender)).unwrap();

        InfallibleOneshotReceiver::from(receiver)
    }
}

//---------------------------------------------------------------------------------------------------- database_writer
/// The main function of the writer thread.
fn database_writer<Req, Res>(
    env: &ConcreteEnv,
    receiver: &crossbeam::channel::Receiver<(Req, oneshot::Sender<DbResult<Res>>)>,
    inner_handler: impl Fn(&ConcreteEnv, &Req) -> DbResult<Res>,
) where
    Req: Send + 'static,
    Res: Debug + Send + 'static,
{
    // 1. Hang on request channel
    // 2. Map request to some database function
    // 3. Execute that function, get the result
    // 4. Return the result via channel
    'main: loop {
        let Ok((request, response_sender)) = receiver.recv() else {
            // If this receive errors, it means that the channel is empty
            // and disconnected, meaning the other side (all senders) have
            // been dropped. This means "shutdown", and we return here to
            // exit the thread.
            //
            // Since the channel is empty, it means we've also processed
            // all requests. Since it is disconnected, it means future
            // ones cannot come in.
            return;
        };

        /// How many times should we retry handling the request on resize errors?
        ///
        /// This is 1 on automatically resizing databases, meaning there is only 1 iteration.
        const REQUEST_RETRY_LIMIT: usize = if ConcreteEnv::MANUAL_RESIZE { 3 } else { 1 };

        // Map [`Request`]'s to specific database functions.
        //
        // Both will:
        // 1. Map the request to a function
        // 2. Call the function
        // 3. (manual resize only) If resize is needed, resize and retry
        // 4. (manual resize only) Redo step {1, 2}
        // 5. Send the function's `Result` back to the requester
        //
        // FIXME: there's probably a more elegant way
        // to represent this retry logic with recursive
        // functions instead of a loop.
        'retry: for retry in 0..REQUEST_RETRY_LIMIT {
            // FIXME: will there be more than 1 write request?
            // this won't have to be an enum.
            let response = inner_handler(env, &request);

            // If the database needs to resize, do so.
            if ConcreteEnv::MANUAL_RESIZE && matches!(response, Err(RuntimeError::ResizeNeeded)) {
                // If this is the last iteration of the outer `for` loop and we
                // encounter a resize error _again_, it means something is wrong.
                assert_ne!(
                    retry, REQUEST_RETRY_LIMIT,
                    "database resize failed maximum of {REQUEST_RETRY_LIMIT} times"
                );

                // Resize the map, and retry the request handling loop.
                //
                // FIXME:
                // We could pass in custom resizes to account for
                // batches, i.e., we're about to add ~5GB of data,
                // add that much instead of the default 1GB.
                // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L665-L695>
                let old = env.current_map_size();
                let new = env.resize_map(None).get();

                const fn bytes_to_megabytes(bytes: usize) -> usize {
                    bytes / 1_000_000
                }

                let old_mb = bytes_to_megabytes(old);
                let new_mb = bytes_to_megabytes(new);

                info!("Resizing database memory map, old: {old_mb}MB, new: {new_mb}MB");

                // Try handling the request again.
                continue 'retry;
            }

            // Automatically resizing databases should not be returning a resize error.
            #[cfg(debug_assertions)]
            if !ConcreteEnv::MANUAL_RESIZE {
                assert!(
                    !matches!(response, Err(RuntimeError::ResizeNeeded)),
                    "auto-resizing database returned a ResizeNeeded error"
                );
            }

            // Send the response back, whether if it's an `Ok` or `Err`.
            if let Err(e) = response_sender.send(response) {
                #[cfg(feature = "tracing")]
                warn!("Database writer failed to send response: {e:?}");
            }

            continue 'main;
        }

        // Above retry loop should either:
        // - continue to the next ['main] loop or...
        // - ...retry until panic
        unreachable!();
    }
}
