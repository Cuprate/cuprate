use cuprate_database::{define_tables, StorableBytes};

use crate::types::{KeyImage, RawCachedVerificationState, TransactionHash, TransactionInfo};

define_tables! {
    0 => TransactionBlobs,
    TransactionHash => StorableBytes,

    1 => TransactionInfomation,
    TransactionHash => TransactionInfo,

    2 => TransactionCachedVerificationState,
    TransactionHash => RawCachedVerificationState,

    3 => SpentKeyImages,
    KeyImage => TransactionHash
}
