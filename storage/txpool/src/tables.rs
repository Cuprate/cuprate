use cuprate_database::{define_tables, StorableVec};

use crate::types::{KeyImage, RawCachedVerificationState, TransactionHash, TransactionInfo};

define_tables! {
    0 => TransactionBlobs,
    TransactionHash => StorableVec<u8>,

    1 => TransactionInformation,
    TransactionHash => TransactionInfo,

    2 => CachedVerificationState,
    TransactionHash => RawCachedVerificationState,

    3 => SpentKeyImages,
    KeyImage => TransactionHash
}
