pub type DbResult<T> = Result<T, BlockchainError>;

/// A blockchain error.
#[derive(thiserror::Error, Debug)]
pub enum BlockchainError {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Fjall(#[from] fjall::Error),
    #[error("not found")]
    NotFound,
}
