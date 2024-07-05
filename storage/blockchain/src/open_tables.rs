//! TODO

//---------------------------------------------------------------------------------------------------- Import
use cuprate_database::{EnvInner, RuntimeError};

use crate::tables::{TablesIter, TablesMut};

//---------------------------------------------------------------------------------------------------- Table function macro
/// `crate`-private macro for callings functions on all tables.
///
/// This calls the function `$fn` with the optional
/// arguments `$args` on all tables - returning early
/// (within whatever scope this is called) if any
/// of the function calls error.
///
/// Else, it evaluates to an `Ok((tuple, of, all, table, types, ...))`,
/// i.e., an `impl Table[Mut]` wrapped in `Ok`.
macro_rules! call_fn_on_all_tables_or_early_return {
    (
        $($fn:ident $(::)?)*
        (
            $($arg:ident),* $(,)?
        )
    ) => {{
        Ok((
            $($fn ::)*<$crate::tables::BlockInfos>($($arg),*)?,
            $($fn ::)*<$crate::tables::BlockBlobs>($($arg),*)?,
            $($fn ::)*<$crate::tables::BlockHeights>($($arg),*)?,
            $($fn ::)*<$crate::tables::KeyImages>($($arg),*)?,
            $($fn ::)*<$crate::tables::NumOutputs>($($arg),*)?,
            $($fn ::)*<$crate::tables::PrunedTxBlobs>($($arg),*)?,
            $($fn ::)*<$crate::tables::PrunableHashes>($($arg),*)?,
            $($fn ::)*<$crate::tables::Outputs>($($arg),*)?,
            $($fn ::)*<$crate::tables::PrunableTxBlobs>($($arg),*)?,
            $($fn ::)*<$crate::tables::RctOutputs>($($arg),*)?,
            $($fn ::)*<$crate::tables::TxBlobs>($($arg),*)?,
            $($fn ::)*<$crate::tables::TxIds>($($arg),*)?,
            $($fn ::)*<$crate::tables::TxHeights>($($arg),*)?,
            $($fn ::)*<$crate::tables::TxOutputs>($($arg),*)?,
            $($fn ::)*<$crate::tables::TxUnlockTime>($($arg),*)?,
        ))
    }};
}
pub(crate) use call_fn_on_all_tables_or_early_return;

//---------------------------------------------------------------------------------------------------- OpenTables
/// Open all tables at once.
///
/// This trait encapsulates the functionality of opening all tables at once.
/// It can be seen as the "constructor" for the [`Tables`](crate::tables::Tables) object.
///
/// Note that this is already implemented on [`cuprate_database::EnvInner`], thus:
/// - You don't need to implement this
/// - It can be called using `env_inner.open_tables()` notation
///
/// # Example
/// ```rust
/// use cuprate_blockchain::{
///     cuprate_database::{Env, EnvInner},
///     config::ConfigBuilder,
///     tables::{Tables, TablesMut},
///     OpenTables,
/// };
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a configuration for the database environment.
/// let tmp_dir = tempfile::tempdir()?;
/// let db_dir = tmp_dir.path().to_owned();
/// let config = ConfigBuilder::new()
///     .db_directory(db_dir.into())
///     .build();
///
/// // Initialize the database environment.
/// let env = cuprate_blockchain::open(config)?;
///
/// // Open up a transaction.
/// let env_inner = env.env_inner();
/// let tx_rw = env_inner.tx_rw()?;
///
/// // Open _all_ tables in write mode using [`OpenTables::open_tables_mut`].
/// // Note how this is being called on `env_inner`.
/// //                        |
/// //                        v
/// let mut tables = env_inner.open_tables_mut(&tx_rw)?;
/// # Ok(()) }
/// ```
pub trait OpenTables<'env> {
    /// The read-only transaction type of the backend.
    type Ro<'a>;
    /// The read-write transaction type of the backend.
    type Rw<'a>;

    /// Open all tables in read/iter mode.
    ///
    /// This calls [`EnvInner::open_db_ro`] on all database tables
    /// and returns a structure that allows access to all tables.
    ///
    /// # Errors
    /// This will only return [`RuntimeError::Io`] if it errors.
    ///
    /// As all tables are created upon [`crate::open`],
    /// this function will never error because a table doesn't exist.
    fn open_tables(&self, tx_ro: &Self::Ro<'_>) -> Result<impl TablesIter, RuntimeError>;

    /// Open all tables in read-write mode.
    ///
    /// This calls [`EnvInner::open_db_rw`] on all database tables
    /// and returns a structure that allows access to all tables.
    ///
    /// # Errors
    /// This will only return [`RuntimeError::Io`] on errors.
    fn open_tables_mut(&self, tx_rw: &Self::Rw<'_>) -> Result<impl TablesMut, RuntimeError>;

    /// Create all database tables.
    ///
    /// This will create all the [`Table`](cuprate_database::Table)s
    /// found in [`tables`](crate::tables).
    ///
    /// # Errors
    /// This will only return [`RuntimeError::Io`] on errors.
    fn create_tables(&self, tx_rw: &Self::Rw<'_>) -> Result<(), RuntimeError>;
}

impl<'env, Ei> OpenTables<'env> for Ei
where
    Ei: EnvInner<'env>,
{
    type Ro<'a> = <Ei as EnvInner<'env>>::Ro<'a>;
    type Rw<'a> = <Ei as EnvInner<'env>>::Rw<'a>;

    fn open_tables(&self, tx_ro: &Self::Ro<'_>) -> Result<impl TablesIter, RuntimeError> {
        call_fn_on_all_tables_or_early_return! {
            Self::open_db_ro(self, tx_ro)
        }
    }

    fn open_tables_mut(&self, tx_rw: &Self::Rw<'_>) -> Result<impl TablesMut, RuntimeError> {
        call_fn_on_all_tables_or_early_return! {
            Self::open_db_rw(self, tx_rw)
        }
    }

    fn create_tables(&self, tx_rw: &Self::Rw<'_>) -> Result<(), RuntimeError> {
        match call_fn_on_all_tables_or_early_return! {
            Self::create_db(self, tx_rw)
        } {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use std::borrow::Cow;

    use cuprate_database::{Env, EnvInner};

    use crate::{config::ConfigBuilder, tests::tmp_concrete_env};

    use super::*;

    /// Tests that [`crate::open`] creates all tables.
    #[test]
    fn test_all_tables_are_created() {
        let (env, _tmp) = tmp_concrete_env();
        let env_inner = env.env_inner();
        let tx_ro = env_inner.tx_ro().unwrap();
        env_inner.open_tables(&tx_ro).unwrap();
    }

    /// Tests that direct usage of
    /// [`cuprate_database::ConcreteEnv`]
    /// does NOT create all tables.
    #[test]
    #[should_panic(expected = "`Result::unwrap()` on an `Err` value: TableNotFound")]
    fn test_no_tables_are_created() {
        let tempdir = tempfile::tempdir().unwrap();
        let config = ConfigBuilder::new()
            .db_directory(Cow::Owned(tempdir.path().into()))
            .low_power()
            .build();
        let env = cuprate_database::ConcreteEnv::open(config.db_config).unwrap();

        let env_inner = env.env_inner();
        let tx_ro = env_inner.tx_ro().unwrap();
        env_inner.open_tables(&tx_ro).unwrap();
    }
}
