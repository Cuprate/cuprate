//! Global `static`s used throughout `cuprated`.

use std::{
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

/// Define all the `static`s that should be always be initialized early on.
///
/// This wraps all `static`s inside a `LazyLock` and generates
/// a [`init_lazylock_statics`] function that must/should be
/// used by `main()` early on.
macro_rules! define_init_lazylock_statics {
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

define_init_lazylock_statics! {
    /// The start time of `cuprated`.
    START_INSTANT: SystemTime = SystemTime::now();

    /// Start time of `cuprated` as a UNIX timestamp.
    START_INSTANT_UNIX: u64 = START_INSTANT
        .duration_since(UNIX_EPOCH)
        .expect("Failed to set `cuprated` startup time.")
        .as_secs();
}

#[cfg(test)]
mod test {
    use super::*;

    /// Sanity check for startup UNIX time.
    #[test]
    fn start_instant_unix() {
        // Fri Sep 27 01:07:13 AM UTC 2024
        assert!(*START_INSTANT_UNIX > 1727399233);
    }
}
