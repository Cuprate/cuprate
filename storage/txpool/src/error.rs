#[derive(thiserror::Error, Debug)]
pub enum TxPoolError {
    #[error("{0}")]
    Fjall(#[from] fjall::Error),
    #[error("Required key not found")]
    NotFound,
}
