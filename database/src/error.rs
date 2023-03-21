
// ------------------------------------------|   Error Enums  |------------------------------------------

#[derive(thiserror::Error, Debug)]
/// `DB_FAILURES` is an enum for backend-agnostic, internal database errors. The `From` Trait must be implemented to the specific backend errors to match DB_FAILURES.
pub enum DB_FAILURES {

	#[error("\n<DB_FAILURES::EncodingError> Failed to encode some data : `{0}`")]
	SerializeIssue(DB_SERIAL),

	#[error("\n<DB::FAILURES::Full> A component is full/busy. The transaction is likely to be restarted  : `{0}`")]
	Full(DB_FULL),

        #[error("\n<DB_FAILURES::KeyAlreadyExist> The database tried to put a key that already exist. Key failed to be insert.")]
        KeyAlreadyExist,

	#[error("\n<DB_FAILURES::FailedToCommit> A transaction tried to commit to the db, but failed.")]
	FailedToCommit,

        #[error("\n<DB_FAILURES::KeyNotFound> The database didn't find the corresponding key.")]
        KeyNotFound,

        #[error("\n<DB_FAILURES::DataNotFound> The database failed to retrieve data section for this key.")]
        DataNotFound,

        #[error("\n<DB_FAILURES::DataSizeLimit> The database was inserting something bigger than the storage engine limit. It shouldn't happen. Please report this issue on github : https://github.com/Cuprate/cuprate/issues")]
        DataSizeLimit,

	#[error("\n<DB_FAILURES::PageNotFound> The database failed to retrieve a page. The database is likely corrupted, search for eventual factors before resyncing again.")]
	PageNotFound,

	#[error("\n<DB_FAILURES::Corrupted> The database tried to fetch a page that is likely corrupted. Please search for eventual reasons before syncing again")]
	PageCorrupted,

	#[error("\n<DB_FAILURES::Panic> The database engine has panic. Please report this issue on github : https://github.com/Cuprate/cuprate/issues")]
	Panic,

	#[error("\n<DB_FAILURES::Undefined, error code: `{0}`> Congratulations you just got an error code we've never think it could exist. Please report this issue on github : https://github.com/Cuprate/cuprate/issues")]
	Undefined(std::ffi::c_int)
}

#[derive(thiserror::Error, Debug)]
pub enum DB_FULL {
	#[error("The specified MaxReaders for database has been reached. The transaction failed to be queued.")]
	ReadTx,

	#[error("All the memory pages have been used. There is no memory left to execute this transaction.")]
	WriteTx,

	#[error("The page being used didn't had enough space. It is an internal error to mdbx. Please report this issue on our github : https://github.com/Cuprate/cuprate/issues")]
	Page,

	#[error("The cursor stacked to deep in its scope.")]
	Cursor,
}

#[derive(thiserror::Error, Debug)]
pub enum DB_SERIAL {
	#[error("An object failed to be serialized into bytes. It is likely an issue from monero-rs library. Please report this error on cuprate's github : https://github.com/Cuprate/cuprate/issues")]
	ConsensusEncode,

	#[error("Bytes failed to be deserialized into the requested object. It is likely an issue from monero-rs library. Please report this error on cuprate's github : https://github.com/Cuprate/cuprate/issues")]
	ConsensusDecode(Box<Vec<u8>>),

	#[error("The database failed to encode bytes in the memory page.")]
	InternalDBEncode(Box<Vec<u8>>),

	#[error("The database failed to decode bytes from the memory page.")]
	InternalDBDecode(Box<Vec<u8>>),
}
