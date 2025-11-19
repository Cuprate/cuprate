pub type DbResult<T> = Result<T, BlockchainError>;

#[derive(thiserror::Error, Debug)]
pub enum BlockchainError {
    #[error(transparent)]
    Heed(#[from] heed::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("not found")]
    NotFound,
}
