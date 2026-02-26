//! [`tower::Service`] integration + thread-pool.
//!
//! ## `service`
//! The `service` module implements the [`tower`] integration,
//! along with the reader/writer thread-pool system.
//!
//! The thread-pool allows outside crates to communicate with it by
//! sending database [`Request`][req_r]s and receiving [`Response`][resp]s `async`hronously -
//! without having to actually worry and handle the database themselves.
//!
//! The system is managed by this crate, and only requires init by the user.
//!
//! ## Handles
//! The 2 handles to the database are:
//! - [`TxpoolReadHandle`]
//! - [`TxpoolWriteHandle`]
//!
//! The 1st allows any caller to send [`ReadRequest`][req_r]s.
//!
//! The 2nd allows any caller to send [`WriteRequest`][req_w]s.
//!
//! Both the handles are cheaply [`Clone`]able.
//!
//! ## Initialization
//! The database & thread-pool system can be initialized with [`init_with_pool()`].
//!
//! This causes the underlying database/threads to be setup
//! and returns a read/write handle to that database.
//!
//! ## Shutdown
//! Upon the above handles being dropped, the corresponding thread(s) will automatically exit, i.e:
//! - The last [`TxpoolReadHandle`] is dropped => reader thread-pool exits
//! - The last [`TxpoolWriteHandle`] is dropped => writer thread exits
//!
//! ## Request and Response
//! To interact with the database (whether reading or writing data),
//! a `Request` can be sent using one of the above handles.
//!
//! Both the handles implement [`tower::Service`], so they can be [`tower::Service::call`]ed.
//!
//! An `async`hronous channel will be returned from the call.
//! This channel can be `.await`ed upon to (eventually) receive
//! the corresponding `Response` to your `Request`.
//!
//! [req_r]: interface::TxpoolReadRequest
//!
//! [req_w]: interface::TxpoolWriteRequest
//!
//! // TODO: we have 2 responses
//!
//! [resp]: interface::TxpoolWriteResponse
//!

mod free;
pub mod interface;
mod read;
mod write;

pub use free::init_with_pool;
pub use read::TxpoolReadHandle;
pub use write::TxpoolWriteHandle;
