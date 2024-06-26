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
//! Upon dropping the [`cuprate_database::ConcreteEnv`]:
//! - All un-processed database transactions are completed
//! - All data gets flushed to disk (caused by [`Drop::drop`] impl on `ConcreteEnv`)
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
//! [req_r]: cuprate_types::blockchain::BCReadRequest
//!
//! [req_w]: cuprate_types::blockchain::BCWriteRequest
//!
//! [resp]: cuprate_types::blockchain::BCResponse
//!
//! # Example
//! Simple usage of `service`.
//!
//! ```rust
//! use hex_literal::hex;
//! use tower::{Service, ServiceExt};
//!
//! use cuprate_types::blockchain::{BCReadRequest, BCWriteRequest, BCResponse};
//! use cuprate_test_utils::data::block_v16_tx0;
//!
//! use cuprate_blockchain::{
//!     cuprate_database::Env,
//!     config::ConfigBuilder,
//! };
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a configuration for the database environment.
//! let tmp_dir = tempfile::tempdir()?;
//! let db_dir = tmp_dir.path().to_owned();
//! let config = ConfigBuilder::new()
//!     .db_directory(db_dir.into())
//!     .build();
//!
//! // Initialize the database thread-pool.
//! let (mut read_handle, mut write_handle) = cuprate_blockchain::service::init(config)?;
//!
//! // Prepare a request to write block.
//! let mut block = block_v16_tx0().clone();
//! # block.height = 0 as u64; // must be 0th height or panic in `add_block()`
//! let request = BCWriteRequest::WriteBlock(block);
//!
//! // Send the request.
//! // We receive back an `async` channel that will
//! // eventually yield the result when `service`
//! // is done writing the block.
//! let response_channel = write_handle.ready().await?.call(request);
//!
//! // Block write was OK.
//! let response = response_channel.await?;
//! assert_eq!(response, BCResponse::WriteBlockOk);
//!
//! // Now, let's try getting the block hash
//! // of the block we just wrote.
//! let request = BCReadRequest::BlockHash(0);
//! let response_channel = read_handle.ready().await?.call(request);
//! let response = response_channel.await?;
//! assert_eq!(
//!     response,
//!     BCResponse::BlockHash(
//!         hex!("43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428")
//!     )
//! );
//!
//! // This causes the writer thread on the
//! // other side of this handle to exit...
//! drop(write_handle);
//! // ...and this causes the reader thread-pool to exit.
//! drop(read_handle);
//! # Ok(()) }
//! ```

mod read;
pub use read::DatabaseReadHandle;

mod write;
pub use write::DatabaseWriteHandle;

mod free;
pub use free::init;

// Internal type aliases for `service`.
mod types;

#[cfg(test)]
mod tests;
