//! Global `static`s used throughout `cuprated`.

use std::{
    sync::{atomic::AtomicU64, LazyLock},
    time::{SystemTime, UNIX_EPOCH},
};

/// Define all the `static`s in the file/module.
///
/// This wraps all `static` inside a `LazyLock` and creates a
/// [`init_lazylock_statics`] function that must/should be
/// used by `main()` early on.
macro_rules! define_lazylock_statics {
	($(
		$( #[$attr:meta] )*
		$name:ident: $t:ty = $init_fn:expr;
	)*) => {
    	/// Initialize global static `LazyLock` data.
		pub fn init_lazylock_statics() {
			$(
				LazyLock::force(&$name);
			)*
		}

		$(
			$(#[$attr])*
			pub static $name: LazyLock<$t> = LazyLock::new(|| $init_fn);
		)*
	};
}

define_lazylock_statics! {
    /// The start time of `cuprated`.
    ///
    /// This must/should be set early on in `main()`.
    START_INSTANT: SystemTime = SystemTime::now();

    /// Start time of `cuprated` as a UNIX timestamp.
    START_INSTANT_UNIX: u64 = START_INSTANT
        .duration_since(UNIX_EPOCH)
        .expect("Failed to set `cuprated` startup time.")
        .as_secs();
}
