//! # Shutdown Flag Module
//!
//! This module provides a global flag to indicate if the application is shutting down.
//! It uses an atomic boolean value (`IS_SHUTTING_DOWN`) to represent the shutdown status,
//! and two public functions, `is_shutting_down()` and `set_shutting_down()`, to access and
//! modify the flag.
//!
//! ## Usage
//!
//! ```rust
//! use shutdown_flag::{is_shutting_down, set_shutting_down};
//!
//! // Check if the application is shutting down
//! if is_shutting_down() {
//!     // Perform shutdown-related tasks
//! }
//!
//! // Set the application shutdown status to `true`
//! set_shutting_down();
//! ```

use std::sync::atomic::{AtomicBool, Ordering};

static IS_SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// Returns true if the application is shutting down.
pub fn is_shutting_down() -> bool {
    // Using `Ordering::Acquire` ensures that all operations in the current
    // thread that come after the `load` won't be moved before it. This guarantees
    // that if the shutdown flag is set, the current thread will see the updated value.
    //
    // In this specific case, using `Acquire` ordering is sufficient because the main
    // purpose is to check the shutdown status, and it is not involved in complex
    // synchronization between threads.
    IS_SHUTTING_DOWN.load(Ordering::Acquire)
}

/// Sets the Cuprate shutdown flag to `true`.
pub fn set_shutting_down() {
    // Using `Ordering::Release` ensures that all operations in the current
    // thread that come before the `store` won't be moved after it. This guarantees
    // that when the shutdown flag is set, other threads will see the updated value
    // when they use `Acquire` ordering to load the flag.
    //
    // In this specific case, using `Release` ordering is sufficient because the main
    // purpose is to set the shutdown status, and it is not involved in complex
    // synchronization between threads.
    IS_SHUTTING_DOWN.store(true, Ordering::Release);
}
