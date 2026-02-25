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
//! The system is managed by this crate, and only requires initialisation by the user.
//!
//! ## Handles
//! The 2 handles to the database are:
//! - [`BlockchainReadHandle`]
//! - [`BlockchainWriteHandle`]
//!
//! The 1st allows any caller to send [`ReadRequest`][req_r]s.
//!
//! The 2nd allows any caller to send [`WriteRequest`][req_w]s.
//!
//! The [`BlockchainReadHandle`] can be shared as it is cheaply [`Clone`]able, however,
//! the [`BlockchainWriteHandle`] cannot be cloned. There is only 1 place in Cuprate that
//! writes, so it is passed there and used.
//!
//! ## Initialization
//! The database can be initialized with [`init_with_pool()`].
//!
//! This causes the underlying database/threads to be setup
//! and returns a read/write handle to that database.
//!
//! ## Shutdown
//! Upon the above handles being dropped, the corresponding thread(s) will automatically exit, i.e:
//! - The last [`BlockchainReadHandle`] is dropped => reader thread-pool exits
//! - The last [`BlockchainWriteHandle`] is dropped => writer thread exits
//!
//! Upon dropping the [`BlockchainDatabase`]:
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
//! [req_r]: cuprate_types::blockchain::BlockchainReadRequest
//!
//! [req_w]: cuprate_types::blockchain::BlockchainWriteRequest
//!
//! [resp]: cuprate_types::blockchain::BlockchainResponse
//!

// needed for docs
use crate::error::DbResult;
use cuprate_types::blockchain::BlockchainResponse;
use tower as _;

mod read;

mod write;
pub use read::BlockchainReadHandle;
pub use write::{init_write_service, BlockchainWriteHandle};

mod free;
use crate::BlockchainDatabase;
pub use free::init_with_pool;

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [`BlockchainResponse`], or a database error occurred.
pub(super) type ResponseResult = DbResult<BlockchainResponse>;
