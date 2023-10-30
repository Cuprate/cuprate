// Abstract Database Trait
//
// This is meant to be the 1st layer that
// abstracts across different databases.
//
// Subsequent specialized layers are built on-top of this trait.
//
// Builds directly on-top of either:
// - An actual db interface
// - Rust bindings to a db interface
pub trait Database {}
