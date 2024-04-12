//! Database writer thread definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::channel::oneshot;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_types::{
    service::{Response, WriteRequest},
    VerifiedBlockInformation,
};

use crate::{error::RuntimeError, ConcreteEnv, Env};

//---------------------------------------------------------------------------------------------------- Constants
/// Name of the writer thread.
const WRITER_THREAD_NAME: &str = concat!(module_path!(), "::DatabaseWriter");

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [Response], or a database error occurred.
type ResponseResult = Result<Response, RuntimeError>;

/// The `Receiver` channel that receives the write response.
///
/// The channel itself should never fail,
/// but the actual database operation might.
type ResponseReceiver = InfallibleOneshotReceiver<ResponseResult>;

/// The `Sender` channel for the response.
type ResponseSender = oneshot::Sender<ResponseResult>;

//---------------------------------------------------------------------------------------------------- DatabaseWriteHandle
/// Write handle to the database.
///
/// This is handle that allows `async`hronously writing to the database,
/// it is not [`Clone`]able as there is only ever 1 place within Cuprate
/// that writes.
///
/// Calling [`tower::Service::call`] with a [`DatabaseWriteHandle`] & [`WriteRequest`]
/// will return an `async`hronous channel that can be `.await`ed upon
/// to receive the corresponding [`Response`].
#[derive(Debug)]
pub struct DatabaseWriteHandle {
    /// Sender channel to the database write thread-pool.
    ///
    /// We provide the response channel for the thread-pool.
    pub(super) sender: crossbeam::channel::Sender<(WriteRequest, ResponseSender)>,
}

impl DatabaseWriteHandle {
    /// Initialize the single `DatabaseWriter` thread.
    #[cold]
    #[inline(never)] // Only called once.
    pub(super) fn init(env: Arc<ConcreteEnv>) -> Self {
        // Initialize `Request/Response` channels.
        let (sender, receiver) = crossbeam::channel::unbounded();

        // Spawn the writer.
        std::thread::Builder::new()
            .name(WRITER_THREAD_NAME.into())
            .spawn(move || {
                let this = DatabaseWriter {
                    receiver,
                    env,
                    env_memory_map_bytes_left: 0,
                };
                DatabaseWriter::main(this);
            })
            .unwrap();

        Self { sender }
    }
}

impl tower::Service<WriteRequest> for DatabaseWriteHandle {
    type Response = Response;
    type Error = RuntimeError;
    type Future = ResponseReceiver;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, request: WriteRequest) -> Self::Future {
        // Response channel we `.await` on.
        let (response_sender, receiver) = oneshot::channel();

        // Send the write request.
        self.sender.send((request, response_sender)).unwrap();

        InfallibleOneshotReceiver::from(receiver)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseWriter
/// The single database writer thread.
pub(super) struct DatabaseWriter {
    /// Receiver side of the database request channel.
    ///
    /// Any caller can send some requests to this channel.
    /// They send them alongside another `Response` channel,
    /// which we will eventually send to.
    receiver: crossbeam::channel::Receiver<(WriteRequest, ResponseSender)>,

    /// Access to the database.
    env: Arc<ConcreteEnv>,

    /// How many free bytes are left in our memory map?
    ///
    /// This is only used if `ConcreteEnv::MANUAL_RESIZE == true`.
    env_memory_map_bytes_left: usize,
}

impl Drop for DatabaseWriter {
    fn drop(&mut self) {
        // TODO: log the writer thread has exited?
    }
}

impl DatabaseWriter {
    /// The `DatabaseWriter`'s main function.
    ///
    /// The writer just loops in this function.
    #[cold]
    #[inline(never)] // Only called once.
    #[allow(unused_mut)] // `self` needs to be mutable for manually resizing DBs
    fn main(mut self) {
        // 1. Hang on request channel
        // 1b. (manual resize only) If resize is needed, resize
        // 2. Map request to some database function
        // 3. Execute that function, get the result
        // 4. Return the result via channel
        loop {
            let Ok((request, response_sender)) = self.receiver.recv() else {
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

            // Calculate if we need to resize the memory map.
            if ConcreteEnv::MANUAL_RESIZE {
                // TODO: calculate the exact byte size needed to store this block.
                // I think a adding a slight buffer is ok since we're only taking
                // up the memory map, not actual disk space.
                //
                // We just need to make sure LMDB never returns a resize error.
                let block_byte_size_including_heap_memory: usize = todo!();

                // If our memory map doesn't have enough bytes to store the block, resize.
                if block_byte_size_including_heap_memory > self.env_memory_map_bytes_left {
                    let map_byte_size_before = self.env.current_map_size();
                    let map_byte_size_after = self.env.resize_map(None);

                    // Add on the bytes we adjusted to.
                    self.env_memory_map_bytes_left +=
                        map_byte_size_after.get() - map_byte_size_before;
                }
            }

            // Map [`Request`]'s to specific database functions.
            //
            // TODO: will there be more than 1 write request?
            // this won't have to be an enum.
            let response = match &request {
                WriteRequest::WriteBlock(block) => write_block(&self.env, block),
            };

            // INVARIANT: We proactively resized above, this error should never be returned.
            #[allow(clippy::manual_assert)]
            #[cfg(debug_assertions)]
            if ConcreteEnv::MANUAL_RESIZE && matches!(response, Err(RuntimeError::ResizeNeeded)) {
                panic!("the database was proactively resized, yet a resize error was returned");
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
            response_sender.send(response).unwrap();
        }
    }

    /// Resize the database's memory map.
    fn resize_map(&self) {
        // The compiler most likely optimizes out this
        // entire function call if this returns here.
        if !ConcreteEnv::MANUAL_RESIZE {
            return;
        }

        // INVARIANT:
        // [`Env`]'s that are `MANUAL_RESIZE` are expected to implement
        // their internals such that we have exclusive access when calling
        // this function. We do not handle the exclusion part, `resize_map()`
        // itself does. The `heed` backend does this with `RwLock`.
        //
        // We need mutual exclusion due to:
        // <http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5>
        self.env.resize_map(None);
        // TODO:
        // We could pass in custom resizes to account for
        // batch transactions, i.e., we're about to add ~5GB
        // of data, add that much instead of the default 1GB.
        // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L665-L695>
    }
}

//---------------------------------------------------------------------------------------------------- Handler functions
// These are the actual functions that do stuff according to the incoming [`Request`].
//
// Each function name is a 1-1 mapping (from CamelCase -> snake_case) to
// the enum variant name, e.g: `BlockExtendedHeader` -> `block_extended_header`.
//
// Each function will return the [`Response`] that we
// should send back to the caller in [`map_request()`].

/// [`WriteRequest::WriteBlock`].
#[inline]
#[allow(clippy::needless_pass_by_value)] // TODO: remove me
fn write_block(env: &ConcreteEnv, block: &VerifiedBlockInformation) -> ResponseResult {
    todo!()
}
