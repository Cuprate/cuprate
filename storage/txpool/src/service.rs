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
//! The database & thread-pool system can be initialized with [`init()`].
//!
//! This causes the underlying database/threads to be setup
//! and returns a read/write handle to that database.
//!
//! ## Shutdown
//! Upon the above handles being dropped, the corresponding thread(s) will automatically exit, i.e:
//! - The last [`TxpoolReadHandle`] is dropped => reader thread-pool exits
//! - The last [`TxpoolWriteHandle`] is dropped => writer thread exits
//!
//! Upon dropping the [`cuprate_database::Env`]:
//! - All un-processed database transactions are completed
//! - All data gets flushed to disk (caused by [`Drop::drop`] impl on `Env`)
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
//! # Example
//! Simple usage of `service`.
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use hex_literal::hex;
//! use tower::{Service, ServiceExt};
//!
//! use cuprate_test_utils::data::TX_V1_SIG2;
//!
//! use cuprate_txpool::{
//!     cuprate_database::Env,
//!     config::ConfigBuilder,
//!     service::interface::{
//!         TxpoolWriteRequest,
//!         TxpoolWriteResponse,
//!         TxpoolReadRequest,
//!         TxpoolReadResponse
//!     }
//! };
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a configuration for the database environment.
//! use cuprate_test_utils::data::TX_V1_SIG2;
//! let tmp_dir = tempfile::tempdir()?;
//! let db_dir = tmp_dir.path().to_owned();
//! let config = ConfigBuilder::new()
//!     .data_directory(db_dir.into())
//!     .build();
//!
//! // Initialize the database thread-pool.
//! let (mut read_handle, mut write_handle, _) = cuprate_txpool::service::init(config)?;
//!
//! // Prepare a request to write block.
//! let tx = TX_V1_SIG2.clone();
//! let request = TxpoolWriteRequest::AddTransaction {
//!     tx: tx.try_into().unwrap(),
//!     state_stem: false,
//! };
//!
//! // Send the request.
//! // We receive back an `async` channel that will
//! // eventually yield the result when `service`
//! // is done writing the tx.
//! let response_channel = write_handle.ready().await?.call(request);
//!
//! // Block write was OK.
//! let TxpoolWriteResponse::AddTransaction(double_spent) = response_channel.await? else {
//!     panic!("tx-pool returned wrong response!");
//! };
//! assert!(double_spent.is_none());
//!
//! // Now, let's try getting the block hash
//! // of the block we just wrote.
//! let request = TxpoolReadRequest::TxBlob(TX_V1_SIG2.tx_hash);
//! let response_channel = read_handle.ready().await?.call(request);
//! let response = response_channel.await?;
//!
//! // This causes the writer thread on the
//! // other side of this handle to exit...
//! drop(write_handle);
//! // ...and this causes the reader thread-pool to exit.
//! drop(read_handle);
//! # Ok(()) }
//! ```

mod free;
pub mod interface;
mod read;
mod types;
mod write;

pub use free::{init, init_with_pool};
pub use types::{TxpoolReadHandle, TxpoolWriteHandle};
