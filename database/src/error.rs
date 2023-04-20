//! ### Error module
//! This module contains all errors abstraction used by the database crate. By implementing [`From<E>`] to the specific errors of storage engine crates, it let us
//! handle more easily any type of error that can happen. This module does **NOT** contain interpretation of these errors, as these are defined for Blockchain abstraction. This is another difference
//! from monerod which interpret these errors directly in its database functions:
//! ```cpp
//! /**
//! * @brief A base class for BlockchainDB exceptions
//! */
//! class DB_EXCEPTION : public std::exception
//! ```
//! see `blockchain_db/blockchain_db.h` in monerod `src/` folder for more details.

#[derive(thiserror::Error, Debug)]
/// `DB_FAILURES` is an enum for backend-agnostic, internal database errors. The `From` Trait must be implemented to the specific backend errors to match DB_FAILURES.
pub enum DB_FAILURES {
    #[error("MDBX returned an error {0}")]
    MDBX_Error(#[from] libmdbx::Error),

    #[error("\n<DB_FAILURES::EncodingError> Failed to encode some data : `{0}`")]
    SerializeIssue(DB_SERIAL),

    #[error("\nObject already exist in the database : {0}")]
    AlreadyExist(&'static str),

    #[error("NotFound? {0}")]
    NotFound(&'static str),

    #[error("\n<DB_FAILURES::Other> `{0}`")]
    Other(&'static str),

    #[error(
        "\n<DB_FAILURES::FailedToCommit> A transaction tried to commit to the db, but failed."
    )]
    FailedToCommit,
}

#[derive(thiserror::Error, Debug)]
pub enum DB_SERIAL {
    #[error("An object failed to be serialized into bytes. It is likely an issue from monero-rs library. Please report this error on cuprate's github : https://github.com/Cuprate/cuprate/issues")]
    ConsensusEncode,

    #[error("Bytes failed to be deserialized into the requested object. It is likely an issue from monero-rs library. Please report this error on cuprate's github : https://github.com/Cuprate/cuprate/issues")]
    ConsensusDecode(Vec<u8>),

    #[error("monero-rs encoding|decoding logic failed : {0}")]
    MoneroEncode(#[from] monero::consensus::encode::Error),

    #[error("Bincode failed to decode a type from the database : {0}")]
    BincodeDecode(#[from] bincode::error::DecodeError),

    #[error("Bincode failed to encode a type for the database : {0}")]
    BincodeEncode(#[from] bincode::error::EncodeError),
}
