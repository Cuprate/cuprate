//! Killswitch.
//!
//! This module implements code for shutting down `cuprated`
//! after a certain timestamp has passed.
//!
//! The reasoning is twofold:
//! 1. Limiting the effects of any network errors
//!    caused by a faulty `cuprated`.
//! 2. To enforce users to update `alpha` builds,
//!    if they choose to run them.
//!
//! This behavior is limited to an alpha build;
//! this module will be removed after a stable v1 release.

use std::{process::exit, time::Duration};

use cuprate_helper::time::current_unix_timestamp;

/// Assert that this is not a v1 release and an alpha release.
pub const _: () = {
    /// Major version number of `cuprated`.
    const MAJOR_VERSION: &str = const_format::str_get!(env!("CARGO_PKG_VERSION"), 0..1).unwrap();

    const_format::assertcp_ne!(
        MAJOR_VERSION,
        "1",
        "`cuprated` major version is 1, killswitch module should be deleted."
    );
};

/// The killswitch activates if the current timestamp is ahead of this timestamp.
///
/// Sat Mar 01 2025 05:00:00 GMT+0000
const KILLSWITCH_ACTIVATION_TIMESTAMP: u64 = 1740805200;

/// Check if the system clock is past a certain timestamp,
/// if so, exit the entire program.
pub fn killswitch() {
    /// A timestamp known to have been passed.
    ///
    /// This is an arbitrary timestamp used for
    /// sanity checking the system's clock to make
    /// sure it is not overly behind.
    ///
    /// Fri Jan 17 2025 14:19:10 GMT+0000
    const SYSTEM_CLOCK_SANITY_TIMESTAMP: u64 = 1737123550;

    /// Print a generic killswitch message.
    fn print_killswitch_msg(msg: String) {
        eprintln!("killswitch: {msg}. `cuprated` will now exit. For more details on why this exists, see: <https://github.com/Cuprate/cuprate/blob/main/binaries/cuprated/src/killswitch.rs>.");
    }

    let current_ts = current_unix_timestamp();

    if current_ts < SYSTEM_CLOCK_SANITY_TIMESTAMP {
        print_killswitch_msg(format!("The system clock is too far behind (current_ts: {current_ts}) and is not reliable to use"));
        exit(66);
    }

    if current_ts > KILLSWITCH_ACTIVATION_TIMESTAMP {
        print_killswitch_msg(format!("The killswitch activation timestamp for alpha builds has passed (current_ts: {current_ts}, killswitch_activation_timestamp: {KILLSWITCH_ACTIVATION_TIMESTAMP})"));
        exit(88);
    }
}

/// Spawn a thread that sleeps until the [`KILLSWITCH_ACTIVATION_TIMESTAMP`] activates.
pub fn init_killswitch() {
    // Check if we should exit immediately.
    killswitch();

    // Else spawn a thread that waits until we should.
    std::thread::spawn(|| -> ! {
        // Sleep until killswitch activation.
        let current_ts = current_unix_timestamp();
        let sleep_duration = Duration::from_secs(KILLSWITCH_ACTIVATION_TIMESTAMP - current_ts);
        std::thread::sleep(sleep_duration);

        // To account for any miscalculated or drifted sleep time,
        // loop until the killswitch activates.
        loop {
            killswitch();
            std::thread::sleep(Duration::from_secs(30));
        }
    });
}
