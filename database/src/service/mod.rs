//! [`tower::Service`] integeration + thread-pool.
//!
//! ## `service`
//! The `service` module implements the [`tower`] integration,
//! along with the reader/writer thread-pool system.
//!
//! The thread-pool allows outside crates to communicate with it by
//! sending database [`Request`][req_r]s and receiving [`Response`][resp]s `async`hronously -
//! without having to actually worry and handle the database themselves.
//!
//! The system is managed by this crate, and only requires [`init`] by the user.
//!
//! This module must be enabled with the `service` feature.
//!
//! ## Handles
//! The 2 handles to the database are:
//! - [`DatabaseReadHandle`]
//! - [`DatabaseWriteHandle`]
//!
//! The 1st allows any caller to send [`ReadRequest`][req_r]s.
//!
//! The 2nd allows any caller to send [`WriteRequest`][req_w]s.
//!
//! The `DatabaseReadHandle` can be shared as it is cheaply [`Clone`]able, however,
//! the `DatabaseWriteHandle` cannot be cloned. There is only 1 place in Cuprate that
//! writes, so it is passed there and used.
//!
//! ## Initialization
//! The database & thread-pool system can be initialized with [`init()`].
//!
//! This causes the underlying database/threads to be setup
//! and returns a read/write handle to that database.
//!
//! ## Shutdown
//! Upon the above handles being dropped, the corresponding thread(s) will automatically exit, i.e:
//! - The last [`DatabaseReadHandle`] is dropped => reader thread-pool exits
//! - The last [`DatabaseWriteHandle`] is dropped => writer thread exits
//!
//! Upon dropping the [`crate::ConcreteEnv`]:
//! - All un-processed database transactions are completed
//! - All data gets flushed to disk (caused by [`Drop::drop`] impl on [`crate::ConcreteEnv`])
//!
//! ## Request and Response
//! To interact with the database (whether reading or writing data),
//! a `Request` can be sent using one of the above handles.
//!
//! Both the handles implement `tower::Service`, so they can be [`tower::Service::call`]ed.
//!
//! An `async`hronous channel will be returned from the call.
//! This channel can be `.await`ed upon to (eventually) receive
//! the corresponding `Response` to your `Request`.
//!
//!
//!
//! [req_r]: cuprate_types::service::ReadRequest
//!
//! [req_w]: cuprate_types::service::WriteRequest
//!
//! [resp]: cuprate_types::service::Response

mod read;
pub use read::DatabaseReadHandle;

mod write;
pub use write::DatabaseWriteHandle;

mod free;
pub use free::init;

#[cfg(test)]
mod tests;
