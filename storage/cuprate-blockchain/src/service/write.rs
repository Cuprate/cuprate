//! Database writer thread definitions and logic.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::channel::oneshot;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_types::{
    service::{BCResponse, BCWriteRequest},
    VerifiedBlockInformation,
};

use crate::{
    env::{Env, EnvInner},
    error::RuntimeError,
    service::types::{ResponseReceiver, ResponseResult, ResponseSender},
    transaction::TxRw,
    ConcreteEnv,
};

//---------------------------------------------------------------------------------------------------- Constants
/// Name of the writer thread.
const WRITER_THREAD_NAME: &str = concat!(module_path!(), "::DatabaseWriter");

//---------------------------------------------------------------------------------------------------- DatabaseWriteHandle
/// Write handle to the database.
///
/// This is handle that allows `async`hronously writing to the database,
/// it is not [`Clone`]able as there is only ever 1 place within Cuprate
/// that writes.
///
/// Calling [`tower::Service::call`] with a [`DatabaseWriteHandle`] & [`BCWriteRequest`]
/// will return an `async`hronous channel that can be `.await`ed upon
/// to receive the corresponding [`BCResponse`].
#[derive(Debug)]
pub struct DatabaseWriteHandle {
    /// Sender channel to the database write thread-pool.
    ///
    /// We provide the response channel for the thread-pool.
    pub(super) sender: crossbeam::channel::Sender<(BCWriteRequest, ResponseSender)>,
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
                let this = DatabaseWriter { receiver, env };
                DatabaseWriter::main(this);
            })
            .unwrap();

        Self { sender }
    }
}

impl tower::Service<BCWriteRequest> for DatabaseWriteHandle {
    type Response = BCResponse;
    type Error = RuntimeError;
    type Future = ResponseReceiver;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, request: BCWriteRequest) -> Self::Future {
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
    receiver: crossbeam::channel::Receiver<(BCWriteRequest, ResponseSender)>,

    /// Access to the database.
    env: Arc<ConcreteEnv>,
}

impl Drop for DatabaseWriter {
    fn drop(&mut self) {
        // TODO: log the writer thread has exited?
    }
}

impl DatabaseWriter {
    /// The `DatabaseWriter`'s main function.
    ///
    /// The writer just loops in this function, handling requests forever
    /// until the request channel is dropped or a panic occurs.
    #[cold]
    #[inline(never)] // Only called once.
    fn main(self) {
        // 1. Hang on request channel
        // 2. Map request to some database function
        // 3. Execute that function, get the result
        // 4. Return the result via channel
        'main: loop {
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
                let response = match &request {
                    BCWriteRequest::WriteBlock(block) => write_block(&self.env, block),
                };

                // If the database needs to resize, do so.
                if ConcreteEnv::MANUAL_RESIZE && matches!(response, Err(RuntimeError::ResizeNeeded))
                {
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
                    let old = self.env.current_map_size();
                    let new = self.env.resize_map(None);

                    // TODO: use tracing.
                    println!("resizing database memory map, old: {old}B, new: {new}B");

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
                    // TODO: use tracing.
                    println!("database writer failed to send response: {e:?}");
                }

                continue 'main;
            }

            // Above retry loop should either:
            // - continue to the next ['main] loop or...
            // - ...retry until panic
            unreachable!();
        }
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

/// [`BCWriteRequest::WriteBlock`].
#[inline]
fn write_block(env: &ConcreteEnv, block: &VerifiedBlockInformation) -> ResponseResult {
    let env_inner = env.env_inner();
    let tx_rw = env_inner.tx_rw()?;

    let result = {
        let mut tables_mut = env_inner.open_tables_mut(&tx_rw)?;
        crate::ops::block::add_block(block, &mut tables_mut)
    };

    match result {
        Ok(()) => {
            TxRw::commit(tx_rw)?;
            Ok(BCResponse::WriteBlockOk)
        }
        Err(e) => {
            // INVARIANT: ensure database atomicity by aborting
            // the transaction on `add_block()` failures.
            TxRw::abort(tx_rw)
                .expect("could not maintain database atomicity by aborting write transaction");
            Err(e)
        }
    }
}
