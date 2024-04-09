//! Spent keys.

//---------------------------------------------------------------------------------------------------- Import
use monero_serai::transaction::{Timelock, Transaction};

use cuprate_types::{OutputOnChain, VerifiedBlockInformation};

use crate::{
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    env::EnvInner,
    error::RuntimeError,
    ops::macros::{doc_add_block_inner_invariant, doc_error},
    tables::{
        BlockBlobs, BlockHeights, BlockInfos, KeyImages, NumOutputs, Outputs, PrunableHashes,
        PrunableTxBlobs, PrunedTxBlobs, RctOutputs, Tables, TablesMut, TxHeights, TxIds,
        TxUnlockTime,
    },
    transaction::{TxRo, TxRw},
    types::{
        BlockHash, BlockHeight, BlockInfo, KeyImage, Output, PreRctOutputId, RctOutput, TxHash,
    },
};

//---------------------------------------------------------------------------------------------------- `add_key_image()`
/// Add a [`KeyImage`] to the "spent" set in the database.
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::key_image::*};
/// // TODO
/// ```
#[inline]
pub fn add_key_image(
    table_key_images: &mut impl DatabaseRw<KeyImages>,
    key_image: &KeyImage,
) -> Result<(), RuntimeError> {
    table_key_images.put(key_image, &())
}

//---------------------------------------------------------------------------------------------------- `remove_key_image()`
/// Remove a [`KeyImage`] from the "spent" set in the database.
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::key_image::*};
/// // TODO
/// ```
#[inline]
pub fn remove_key_image(
    table_key_images: &mut impl DatabaseRw<KeyImages>,
    key_image: &KeyImage,
) -> Result<(), RuntimeError> {
    table_key_images.delete(key_image)
}

/// Check if a [`KeyImage`] exists - i.e. if it is "spent".
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::key_image::*};
/// // TODO
/// ```
#[doc = doc_error!()]
#[inline]
pub fn key_image_exists(
    table_key_images: &impl DatabaseRo<KeyImages>,
    key_image: &KeyImage,
) -> Result<bool, RuntimeError> {
    table_key_images.contains(key_image)
}
