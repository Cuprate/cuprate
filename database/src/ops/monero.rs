//! Abstracted Monero database operations; `trait MoneroRo` & `trait MoneroRw`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    ops::{Deref, RangeBounds},
};

use crate::{
    database::{DatabaseRo, DatabaseRw},
    table::Table,
};

//---------------------------------------------------------------------------------------------------- MoneroRo
/// Monero database read operations.
///
/// TODO
pub trait MoneroRo<T: Table>: DatabaseRo<T> {}

//---------------------------------------------------------------------------------------------------- MoneroRw
/// Monero database read/write operations.
///
/// TODO
pub trait MoneroRw<T: Table>: DatabaseRw<T> {}
