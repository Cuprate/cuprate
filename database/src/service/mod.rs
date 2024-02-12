//! [`tower::Service`] integeration + thread-pool.
//!
//! ## `service`
//! The `service` module implements the [`tower`] integration,
//! along with the reader/writer thread-pool system.
//!
//! The thread-pool allows outside crates to communicate with it by
//! sending database [`Request`](ReadRequest)s and receiving [`Response`]s `async`hronously -
//! without having to actually worry and handle the database themselves.
//!
//! The system is managed by this crate, and only
//! requires [`init`] and [`shutdown`] by the user.
//!
//! This module must be enabled with the `service` feature.
//!
//! ## Initialization
//! The database & thread-pool system can be initialized with [`init()`].
//!
//! This causes the underlying database/threads to be setup
//! and returns a read/write handle to that database.
//!
//! ## Handles
//! The 2 handles to the database are:
//! - [`DatabaseReadHandle`]
//! - [`DatabaseWriteHandle`]
//!
//! The 1st allows any caller to send/receive [`ReadRequest`] & [`ReadResponse`].
//!
//! The 2nd allows any caller to send/receive [`WriteRequest`] & [`WriteResponse`].
//!
//! Both of these handles are cheaply [`Clone`]able and can be
//! passed around to whomever needs access to the database.
//!
//! ## Request and Response
//! To interact with the database (whether reading or writing data),
//! a `Request` can be sent using one of the above handles.
//!
//! Both the handles implement `tower::Service`, so they can be [`tower::Service::call`]ed.
//!
//! An `async`hronous channel will be returned from the call.
//! This channel can be `.await`ed upon to (eventually) receive
//! corresponding `Response` to your `Request`.

mod read;
pub use read::DatabaseReadHandle;

mod write;
pub use write::DatabaseWriteHandle;

mod free;
pub use free::{init, shutdown};

mod request;
pub use request::{ReadRequest, WriteRequest};

mod response;
pub use response::{ReadResponse, Response, WriteResponse};

#[cfg(test)]
mod tests;
