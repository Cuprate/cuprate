//! Counting active connections used by Cuprate.
//!
//! These types can be used to count any kind of active resource.
//! But they are currently used to track the number of open connections.

use std::{fmt, sync::Arc};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// A counter for active connections.
///
/// Creates a [`ConnectionTracker`] to track each active connection.
/// When these trackers are dropped, the counter gets notified.
pub struct ActiveConnectionCounter {
    /// The limit for this type of connection, for diagnostics only.
    /// The caller must enforce the limit by ignoring, delaying, or dropping connections.
    limit: usize,

    /// The label for this connection counter, typically its type.
    label: Arc<str>,

    semaphore: Arc<Semaphore>,
}

impl fmt::Debug for ActiveConnectionCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveConnectionCounter")
            .field("label", &self.label)
            .field("count", &self.count())
            .field("limit", &self.limit)
            .finish()
    }
}

impl ActiveConnectionCounter {
    /// Create and return a new active connection counter.
    pub fn new_counter() -> Self {
        Self::new_counter_with(Semaphore::MAX_PERMITS, "Active Connections")
    }

    /// Create and return a new active connection counter with `limit` and `label`.
    /// The caller must check and enforce limits using [`update_count()`](Self::update_count).
    pub fn new_counter_with<S: ToString>(limit: usize, label: S) -> Self {
        let label = label.to_string();

        Self {
            limit,
            label: label.into(),
            semaphore: Arc::new(Semaphore::new(limit)),
        }
    }

    /// Create and return a new [`ConnectionTracker`], using a permit from the semaphore,
    /// SAFETY:
    ///     This function will panic if the semaphore doesn't have anymore permits.
    pub fn track_connection(&mut self) -> ConnectionTracker {
        ConnectionTracker::new(self)
    }

    pub fn count(&self) -> usize {
        let count = self
            .limit
            .checked_sub(self.semaphore.available_permits())
            .expect("Limit is less than available connection permits");

        tracing::trace!(
            open_connections = ?count,
            limit = ?self.limit,
            label = ?self.label,
        );

        count
    }

    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

/// A per-connection tracker.
///
/// [`ActiveConnectionCounter`] creates a tracker instance for each active connection.
pub struct ConnectionTracker {
    /// The permit for this connection, updates the semaphore when dropped.
    permit: OwnedSemaphorePermit,

    /// The label for this connection counter, typically its type.
    label: Arc<str>,
}

impl fmt::Debug for ConnectionTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ConnectionTracker")
            .field(&self.label)
            .finish()
    }
}

impl ConnectionTracker {
    /// Create and return a new active connection tracker, and add 1 to `counter`.
    /// All connection trackers share a label with their connection counter.
    ///
    /// When the returned tracker is dropped, `counter` will be notified.
    ///
    /// SAFETY:
    ///     This function will panic if the [`ActiveConnectionCounter`] doesn't have anymore permits.
    fn new(counter: &mut ActiveConnectionCounter) -> Self {
        tracing::debug!(
            open_connections = ?counter.count(),
            limit = ?counter.limit,
            label = ?counter.label,
            "opening a new peer connection",
        );

        Self {
            permit: counter.semaphore.clone().try_acquire_owned().unwrap(),
            label: counter.label.clone(),
        }
    }
}

impl Drop for ConnectionTracker {
    fn drop(&mut self) {
        tracing::debug!(
            label = ?self.label,
            "A peer connection has closed",
        );
        // the permit is automatically dropped
    }
}