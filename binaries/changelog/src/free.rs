//! Free functions.

use std::process::Command;

use chrono::{DateTime, Utc};

use crate::crates::CuprateCrates;

/// Assert we are at `Cuprate/cuprate`.
///
/// This binary relys on this.
pub fn assert_repo_root() {
    let path = std::env::current_dir().unwrap();

    // Check path.
    assert!(
        path.ends_with("Cuprate/cuprate"),
        "This binary must be ran at the repo root."
    );

    // Sanity check cargo.
    CuprateCrates::new().crate_version("cuprated");
}

pub fn fmt_date(date: &DateTime<Utc>) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn generate_cuprated_help_text() -> String {
    String::from_utf8(
        Command::new("cargo")
            .args(["run", "--bin", "cuprated", "--", "--help"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
}
