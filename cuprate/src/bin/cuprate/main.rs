//! Main entry point for Cuprate

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use cuprate::application::APP;

/// Boot Cuprate
fn main() {
    abscissa_core::boot(&APP);
}
