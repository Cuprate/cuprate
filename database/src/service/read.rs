//! Database read thread-pool definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::task::{Context, Poll};

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{
    config::ReaderThreads,
    error::RuntimeError,
    service::{request::ReadRequest, response::Response},
    ConcreteEnv,
};

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [Response], or a database error occurred.
type ResponseResult = Result<Response, RuntimeError>;

/// The `Receiver` channel that receives the read response.
///
/// This is owned by the caller (the reader)
/// who `.await`'s for the response.
///
/// The channel itself should never fail,
/// but the actual database operation might.
type ResponseRecv = InfallibleOneshotReceiver<ResponseResult>;

/// The `Sender` channel for the response.
///
/// The database reader thread uses this to send
/// the database result to the caller.
type ResponseSend = tokio::sync::oneshot::Sender<ResponseResult>;

//---------------------------------------------------------------------------------------------------- DatabaseReadHandle
/// Read handle to the database.
///
/// This is cheaply [`Clone`]able handle that
/// allows `async`hronously reading from the database.
///
/// Calling [`tower::Service::call`] with a [`DatabaseReadHandle`] & [`ReadRequest`]
/// will return an `async`hronous channel that can be `.await`ed upon
/// to receive the corresponding [`Response`].
#[derive(Clone, Debug)]
pub struct DatabaseReadHandle {
    /// Sender channel to the database read thread-pool.
    ///
    /// We provide the response channel for the thread-pool.
    pub(super) sender: crossbeam::channel::Sender<(ReadRequest, ResponseSend)>,
}

impl tower::Service<ReadRequest> for DatabaseReadHandle {
    type Response = Response;
    type Error = RuntimeError;
    type Future = ResponseRecv;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    #[inline]
    fn call(&mut self, _req: ReadRequest) -> Self::Future {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseReaderShutdown
/// A handle to _all_ database reader threads,
/// and permission to make them all "shutdown"
/// (see [`DatabaseReader`] docs for why these
/// threads don't actually exit).
#[derive(Debug)]
pub(super) struct DatabaseReaderShutdown {
    /// TODO
    shutdown_channels: Vec<crossbeam::channel::Sender<()>>,
}

impl DatabaseReaderShutdown {
    /// TODO
    pub(super) fn shutdown(self) {
        for shutdown_channel in self.shutdown_channels {
            shutdown_channel.try_send(()).unwrap();
        }
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseReader
/// Database reader thread.
///
/// This struct essentially represents a thread.
///
/// Each reader thread is spawned with access to this struct (self).
pub(super) struct DatabaseReader {
    /// Receiver side of the database request channel.
    ///
    /// Any caller can send some requests to this channel.
    /// They send them alongside another `Response` channel,
    /// which we will eventually send to.
    ///
    /// We (the database reader thread) are not responsible
    /// for creating this channel, the caller provides it.
    ///
    /// SOMEDAY: this struct itself could cache a return channel
    /// instead of creating a new `oneshot` each request.
    receiver: crossbeam::channel::Receiver<(ReadRequest, ResponseSend)>,

    /// TODO
    shutdown: crossbeam::channel::Receiver<()>,

    /// Access to the database.
    db: ConcreteEnv,
}

impl DatabaseReader {
    /// Initialize the `DatabaseReader` thread-pool.
    ///
    /// This spawns `N` amount of `DatabaseReader`'s
    /// attached to `db` and returns a handle to the pool.
    ///
    /// Should be called _once_ per actual database.
    #[cold]
    #[inline(never)] // Only called once.
    pub(super) fn init(
        db: &ConcreteEnv,
        reader_threads: ReaderThreads,
    ) -> (DatabaseReadHandle, DatabaseReaderShutdown) {
        // Initialize `Request/Response` channels.
        let (sender, receiver) = crossbeam::channel::unbounded();

        // How many reader threads to spawn?
        let readers = reader_threads.as_threads().get();

        // Collect shutdown channels for each reader thread.
        let mut shutdown_channels = Vec::with_capacity(readers);

        // Spawn pool of readers.
        for _ in 0..readers {
            let receiver = receiver.clone();
            let db = ConcreteEnv::clone(db);
            let (shutdown_send, shutdown) = crossbeam::channel::bounded(1);
            shutdown_channels.push(shutdown_send);

            std::thread::spawn(move || {
                let this = Self {
                    receiver,
                    shutdown,
                    db,
                };

                Self::main(this);
            });
        }

        // Return a handle to the pool and channels to
        // allow clean shutdown of all reader threads.
        (
            DatabaseReadHandle { sender },
            DatabaseReaderShutdown { shutdown_channels },
        )
    }

    /// The `DatabaseReader`'s main function.
    ///
    /// Each thread just loops in this function.
    #[cold]
    #[inline(never)] // Only called once.
    fn main(self) {
        let mut select = crossbeam::channel::Select::new();
        assert_eq!(0, select.recv(&self.receiver));
        assert_eq!(1, select.recv(&self.shutdown));

        // 1. Hang on request channel
        // 2. Map request to some database function
        // 3. Execute that function, get the result
        // 4. Return the result via channel
        loop {
            // Q: Why are we checking and `continue`ing if
            // the channel returned an error?
            //
            // A: Because `select` can return spuriously, so
            // we must double check a channel actually has a message:
            // <https://docs.rs/crossbeam/latest/crossbeam/channel/struct.Select.html#method.ready>
            match select.ready() {
                // Database requests.
                0 => {
                    let Ok((request, response_send)) = self.receiver.try_recv() else {
                        continue;
                    };

                    // Map [`Request`]'s to specific database functions.
                    match request {
                        ReadRequest::Example1 => self.example_handler_1(response_send),
                        ReadRequest::Example2(_x) => self.example_handler_2(response_send),
                        ReadRequest::Example3(_x) => self.example_handler_3(response_send),
                    }
                }

                // Shutdown signal.
                1 => {
                    if self.receiver.try_recv().is_ok() {
                        Self::shutdown(self);
                    };
                }

                // Message from a ghost ooo very spooky.
                _ => unreachable!(),
            }
        }
    }

    /// TODO
    #[inline]
    fn example_handler_1(&self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }

    /// TODO
    #[inline]
    fn example_handler_2(&self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }

    /// TODO
    #[inline]
    fn example_handler_3(&self, response_send: ResponseSend) {
        let db_result = todo!();
        response_send.send(db_result).unwrap();
    }

    /// TODO
    #[cold]
    #[inline(never)]
    fn shutdown(self) -> ! {
        // Drop our strong reference to the database environment.
        // (but not the other stuff in `self`).
        drop(self.db);

        // TODO: (so the channel doesn't get dropped, so mid-requests don't panic)
        loop {
            std::thread::park();
        }
    }
}
