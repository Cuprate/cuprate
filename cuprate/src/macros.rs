// Top-level convenience macros.
//
// Only used within `cuprate`, not visable to outside crates.

//---------------------------------------------------------------------------------------------------- Use

//---------------------------------------------------------------------------------------------------- __NAME__
// The general macro to use when exiting early on failure.
// Not meant for usage after we start initializing into the
// actual node code.
//
// Used on errors very early in the init process, i.e:
// - Conflicting config options
// - Disk problems
// - `--bad-flags`
//
// 1. Print an error message (log/STDERR)
// 2. Exit the entire program WITHOUT running destructors or panic hook
#[macro_export]
macro_rules! exit {
	(
		$code:literal, // Error code
		$($msg:tt),*   // The error message to print
	) => {{
		// TODO(hinto): branch if log is initialized
		if /* logger_is_on */ true {
			// TODO(hinto): replace log function
			::std::eprintln!("cuprate error: {}", ::std::format_args!($($msg)*));
		} else {
			::std::eprintln!("cuprate error: {}", ::std::format_args!($($msg)*));
		}

		::std::process::exit($code);
	}};

	// No error code, default to `1`.
	($($msg:tt),* $(,)?) => {{
		// TODO(hinto): branch if log is initialized
		if /* logger_is_on */ true {
			// TODO(hinto): replace log function
			::std::eprintln!("cuprate error: {}", ::std::format_args!($($msg)*));
		} else {
			::std::eprintln!("cuprate error: {}", ::std::format_args!($($msg)*));
		}

		::std::process::exit(1);
	}}
}
pub(crate) use exit;

//---------------------------------------------------------------------------------------------------- TESTS
//#[cfg(test)]
//mod tests {
//	#[test]
//		fn __TEST__() {
//	}
//}
