//! Database service type aliases.
//!
//! Only used internally for our `tower::Service` impls.

//---------------------------------------------------------------------------------------------------- Use
use futures::channel::oneshot::Sender;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_types::service::BCResponse;

use crate::error::RuntimeError;

//---------------------------------------------------------------------------------------------------- Types
/// The actual type of the response.
///
/// Either our [`BCResponse`], or a database error occurred.
pub(super) type ResponseResult = Result<BCResponse, RuntimeError>;

/// The `Receiver` channel that receives the read response.
///
/// This is owned by the caller (the reader/writer thread)
/// who `.await`'s for the response.
///
/// The channel itself should never fail,
/// but the actual database operation might.
pub(super) type ResponseReceiver = InfallibleOneshotReceiver<ResponseResult>;

/// The `Sender` channel for the response.
///
/// The database reader/writer thread uses this to send the database result to the caller.
pub(super) type ResponseSender = Sender<ResponseResult>;
