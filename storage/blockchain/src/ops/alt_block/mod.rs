mod block;
mod chain;
mod tx;

pub use block::*;
pub use chain::*;
pub use tx::*;

pub fn flush_alt_blocks<'a, E: cuprate_database::EnvInner<'a>>(
    env_inner: &E,
    tx_rw: &mut E::Rw<'_>,
) -> Result<(), cuprate_database::RuntimeError> {
    use crate::tables::{
        AltBlockBlobs, AltBlockHeights, AltBlocksInfo, AltChainInfos, AltTransactionBlobs,
        AltTransactionInfos,
    };

    env_inner.clear_db::<AltChainInfos>(tx_rw)?;
    env_inner.clear_db::<AltBlockHeights>(tx_rw)?;
    env_inner.clear_db::<AltBlocksInfo>(tx_rw)?;
    env_inner.clear_db::<AltBlockBlobs>(tx_rw)?;
    env_inner.clear_db::<AltTransactionBlobs>(tx_rw)?;
    env_inner.clear_db::<AltTransactionInfos>(tx_rw)
}
