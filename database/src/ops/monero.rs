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
    tables::BlockBlobs,
    ConcreteEnv, Env, EnvInner, TxRo, TxRw,
};

//---------------------------------------------------------------------------------------------------- MoneroRo
/// Monero database read operations.
///
/// TODO
pub trait MoneroRo<'env, Ro, Rw>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Self: EnvInner<'env, Ro, Rw>,
{
}

//---------------------------------------------------------------------------------------------------- MoneroRw
/// Monero database read/write operations.
///
/// TODO
pub trait MoneroRw<'env, Ro, Rw>
where
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
    Self: EnvInner<'env, Ro, Rw>,
{
}
