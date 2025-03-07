#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use crate::misc::PoolInfoExtent;
#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    EpeeObject, EpeeObjectBuilder, epee_object, error,
    macros::bytes::{Buf, BufMut},
    read_epee_value, write_field,
};

use cuprate_fixed_bytes::ByteArrayVec;

use crate::misc::PoolTxInfo;

//---------------------------------------------------------------------------------------------------- PoolInfo
#[doc = crate::macros::monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    223..=228
)]
/// Used in [`crate::bin::GetBlocksResponse`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PoolInfo {
    #[default]
    None,
    Incremental(PoolInfoIncremental),
    Full(PoolInfoFull),
}

//---------------------------------------------------------------------------------------------------- Internal data
/// Data within [`PoolInfo::Incremental`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PoolInfoIncremental {
    pub added_pool_txs: Vec<PoolTxInfo>,
    pub remaining_added_pool_txids: ByteArrayVec<32>,
    pub removed_pool_txids: ByteArrayVec<32>,
}

#[cfg(feature = "epee")]
epee_object! {
    PoolInfoIncremental,
    added_pool_txs: Vec<PoolTxInfo>,
    remaining_added_pool_txids: ByteArrayVec<32>,
    removed_pool_txids: ByteArrayVec<32>,
}

/// Data within [`PoolInfo::Full`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PoolInfoFull {
    pub added_pool_txs: Vec<PoolTxInfo>,
    pub remaining_added_pool_txids: ByteArrayVec<32>,
}

#[cfg(feature = "epee")]
epee_object! {
    PoolInfoFull,
    added_pool_txs: Vec<PoolTxInfo>,
    remaining_added_pool_txids: ByteArrayVec<32>,
}

//---------------------------------------------------------------------------------------------------- PoolInfo epee impl
#[cfg(feature = "epee")]
/// [`EpeeObjectBuilder`] for [`GetBlocksResponse`].
///
/// Not for public usage.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct __PoolInfoEpeeBuilder {
    /// This is a distinct field in `monerod`,
    /// which as represented in this library with [`PoolInfo`]'s `u8` tag.
    pub pool_info_extent: Option<PoolInfoExtent>,

    pub added_pool_txs: Option<Vec<PoolTxInfo>>,
    pub remaining_added_pool_txids: Option<ByteArrayVec<32>>,
    pub removed_pool_txids: Option<ByteArrayVec<32>>,
}

// Custom epee implementation.
//
// HACK/INVARIANT:
// If any data within [`PoolInfo`] changes, the below code should be changed as well.
#[cfg(feature = "epee")]
impl EpeeObjectBuilder<PoolInfo> for __PoolInfoEpeeBuilder {
    fn add_field<B: Buf>(&mut self, name: &str, r: &mut B) -> error::Result<bool> {
        macro_rules! read_epee_field {
            ($($field:ident),*) => {
                match name {
                    $(
                        stringify!($field) => { self.$field = Some(read_epee_value(r)?); },
                    )*
                    _ => return Ok(false),
                }
            };
        }

        read_epee_field! {
            pool_info_extent,
            added_pool_txs,
            remaining_added_pool_txids,
            removed_pool_txids
        }

        Ok(true)
    }

    fn finish(self) -> error::Result<PoolInfo> {
        // INVARIANT:
        // `monerod` omits serializing the field itself when a container is empty,
        // `unwrap_or_default()` is used over `error()` in these cases.
        // Some of the uses are when values have default fallbacks: `pool_info_extent`.

        let pool_info_extent = self.pool_info_extent.unwrap_or_default();
        let this = match pool_info_extent {
            PoolInfoExtent::None => PoolInfo::None,
            PoolInfoExtent::Incremental => PoolInfo::Incremental(PoolInfoIncremental {
                added_pool_txs: self.added_pool_txs.unwrap_or_default(),
                remaining_added_pool_txids: self.remaining_added_pool_txids.unwrap_or_default(),
                removed_pool_txids: self.removed_pool_txids.unwrap_or_default(),
            }),
            PoolInfoExtent::Full => PoolInfo::Full(PoolInfoFull {
                added_pool_txs: self.added_pool_txs.unwrap_or_default(),
                remaining_added_pool_txids: self.remaining_added_pool_txids.unwrap_or_default(),
            }),
        };

        Ok(this)
    }
}

#[cfg(feature = "epee")]
impl EpeeObject for PoolInfo {
    type Builder = __PoolInfoEpeeBuilder;

    fn number_of_fields(&self) -> u64 {
        // Inner struct fields.
        let inner_fields = match self {
            Self::None => 0,
            Self::Incremental(s) => s.number_of_fields(),
            Self::Full(s) => s.number_of_fields(),
        };

        // [`PoolInfoExtent`] + inner struct fields
        1 + inner_fields
    }

    fn write_fields<B: BufMut>(self, w: &mut B) -> error::Result<()> {
        const FIELD: &str = "pool_info_extent";

        match self {
            Self::None => {
                write_field(PoolInfoExtent::None.to_u8(), FIELD, w)?;
            }
            Self::Incremental(s) => {
                s.write_fields(w)?;
                write_field(PoolInfoExtent::Incremental.to_u8(), FIELD, w)?;
            }
            Self::Full(s) => {
                s.write_fields(w)?;
                write_field(PoolInfoExtent::Full.to_u8(), FIELD, w)?;
            }
        }

        Ok(())
    }
}
