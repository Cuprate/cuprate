use std::{
    fmt::Debug,
    sync::Arc,
    task::{Context, Poll},
};

use futures::channel::oneshot;
use tracing::{info, warn};

use cuprate_database::{DbResult, Env, RuntimeError};
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
    pub fn init<D: Send + Sync + 'static>(
        env: D,
        inner_handler: impl Fn(&D, &Req) -> DbResult<Res> + Send + 'static,
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
fn database_writer<Req, Res, D>(
    env: &D,
    receiver: &crossbeam::channel::Receiver<(Req, oneshot::Sender<DbResult<Res>>)>,
    inner_handler: impl Fn(&D, &Req) -> DbResult<Res>,
) where
    Req: Send + 'static,
    Res: Debug + Send + 'static,
    D: Send + Sync + 'static,
{
    // 1. Hang on request channel
    // 2. Map request to some database function
    // 3. Execute that function, get the result
    // 4. Return the result via channel
    loop {
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
        
        let response = inner_handler(env, &request);
        
    
        // Send the response back, whether if it's an `Ok` or `Err`.
        if let Err(e) = response_sender.send(response) {
            warn!("Database writer failed to send response: {e:?}");
        }
    }
}
