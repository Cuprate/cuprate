use std::sync::{Arc, Mutex};

use thiserror::Error;
use tracing_error::TracedError;
use monero_wire::BucketError;

/// A wrapper around `Arc<PeerError>` that implements `Error`.
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct SharedPeerError(Arc<TracedError<PeerError>>);

impl<E> From<E> for SharedPeerError
where
    PeerError: From<E>,
{
    fn from(source: E) -> Self {
        Self(Arc::new(TracedError::from(PeerError::from(source))))
    }
}

impl SharedPeerError {
    /// Returns a debug-formatted string describing the inner [`PeerError`].
    ///
    /// Unfortunately, [`TracedError`] makes it impossible to get a reference to the original error.
    pub fn inner_debug(&self) -> String {
        format!("{:?}", self.0.as_ref())
    }
}

#[derive(Debug, Error, Clone)]
pub enum PeerError {
    #[error("The connection task has closed.")]
    ConnectionTaskClosed,
    #[error("The connected peer sent an incorrect response.")]
    PeerSentIncorrectResponse,
    #[error("The connected peer sent an incorrect response.")]
    BucketError(#[from] BucketError)
}

/// A shared error slot for peer errors.
///
/// # Correctness
///
/// Error slots are shared between sync and async code. In async code, the error
/// mutex should be held for as short a time as possible. This avoids blocking
/// the async task thread on acquiring the mutex.
///
/// > If the value behind the mutex is just data, it’s usually appropriate to use a blocking mutex
/// > ...
/// > wrap the `Arc<Mutex<...>>` in a struct
/// > that provides non-async methods for performing operations on the data within,
/// > and only lock the mutex inside these methods
///
/// <https://docs.rs/tokio/1.15.0/tokio/sync/struct.Mutex.html#which-kind-of-mutex-should-you-use>
#[derive(Default, Clone)]
pub struct ErrorSlot(Arc<std::sync::Mutex<Option<SharedPeerError>>>);

impl std::fmt::Debug for ErrorSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // don't hang if the mutex is locked
        // show the panic if the mutex was poisoned
        f.debug_struct("ErrorSlot")
            .field("error", &self.0.try_lock())
            .finish()
    }
}

impl ErrorSlot {
    /// Read the current error in the slot.
    ///
    /// Returns `None` if there is no error in the slot.
    ///
    /// # Correctness
    ///
    /// Briefly locks the error slot's threaded `std::sync::Mutex`, to get a
    /// reference to the error in the slot.
    #[allow(clippy::unwrap_in_result)]
    pub fn try_get_error(&self) -> Option<SharedPeerError> {
        self.0
            .lock()
            .expect("error mutex should be unpoisoned")
            .as_ref()
            .cloned()
    }

    /// Update the current error in the slot.
    ///
    /// Returns `Err(AlreadyErrored)` if there was already an error in the slot.
    ///
    /// # Correctness
    ///
    /// Briefly locks the error slot's threaded `std::sync::Mutex`, to check for
    /// a previous error, then update the error in the slot.
    #[allow(clippy::unwrap_in_result)]
    pub fn try_update_error(&self, e: SharedPeerError) -> Result<(), AlreadyErrored> {
        let mut guard = self.0.lock().expect("error mutex should be unpoisoned");

        if let Some(original_error) = guard.clone() {
            Err(AlreadyErrored { original_error })
        } else {
            *guard = Some(e);
            Ok(())
        }
    }
}

/// Error returned when the [`ErrorSlot`] already contains an error.
#[derive(Clone, Debug)]
pub struct AlreadyErrored {
    /// The original error in the error slot.
    pub original_error: SharedPeerError,
}
