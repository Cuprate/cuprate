use monero_serai::ringct::bulletproofs::BatchVerifier as InternalBatchVerifier;

/// This trait represents a batch verifier.
///
/// A batch verifier is used to speed up verification by verifying multiple transactions together.
///
/// Not all proofs can be batched and at its core it's intended to verify a series of statements are
/// each equivalent to zero.
pub trait BatchVerifier {
    /// Queue a statement for batch verification.
    ///
    /// # Panics
    /// This function may panic if `stmt` contains calls to `rayon`'s parallel iterators, e.g. `par_iter()`.
    // TODO: remove the panics by adding a generic API upstream.
    fn queue_statement<R>(&mut self, stmt: impl FnOnce(&mut InternalBatchVerifier) -> R) -> R;
}

// impl this for a single threaded batch verifier.
impl BatchVerifier for &'_ mut InternalBatchVerifier {
    fn queue_statement<R>(&mut self, stmt: impl FnOnce(&mut InternalBatchVerifier) -> R) -> R {
        stmt(self)
    }
}
