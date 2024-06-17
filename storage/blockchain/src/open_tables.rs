//! TODO

//---------------------------------------------------------------------------------------------------- Import
use cuprate_database::{EnvInner, RuntimeError, TxRo, TxRw};

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

//---------------------------------------------------------------------------------------------------- OpenTables
/// TODO
pub trait OpenTables<'env, Ro, Rw>
where
    Self: 'env,
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
{
    /// TODO
    ///
    /// # Errors
    /// TODO
    fn open_tables(&'env self, tx_ro: &Ro) -> Result<impl TablesIter, RuntimeError>;

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn open_tables_mut(&'env self, tx_rw: &Rw) -> Result<impl TablesMut, RuntimeError>;
}

impl<'env, Ei, Ro, Rw> OpenTables<'env, Ro, Rw> for Ei
where
    Ei: EnvInner<'env, Ro, Rw>,
    Ro: TxRo<'env>,
    Rw: TxRw<'env>,
{
    fn open_tables(&'env self, tx_ro: &Ro) -> Result<impl TablesIter, RuntimeError> {
        call_fn_on_all_tables_or_early_return! {
            Self::open_db_ro(self, tx_ro)
        }
    }

    fn open_tables_mut(&'env self, tx_rw: &Rw) -> Result<impl TablesMut, RuntimeError> {
        call_fn_on_all_tables_or_early_return! {
            Self::open_db_rw(self, tx_rw)
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
