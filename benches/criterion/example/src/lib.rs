#![doc = include_str!("../README.md")] // See the README for crate documentation.
#![allow(unused_crate_dependencies, reason = "used in benchmarks")]

/// Shared type that all benchmarks can use.
#[expect(dead_code)]
pub struct SomeHardToCreateObject(u64);

impl From<u64> for SomeHardToCreateObject {
    /// Shared function that all benchmarks can use.
    fn from(value: u64) -> Self {
        Self(value)
    }
}
