use multiexp::BatchVerifier as InternalBatchVerifier;

/// This trait represents a batch verifier.
pub trait BatchVerifier {
    /// Queue a statement for batch verification.
    ///
    /// # Panics
    /// This function may panic if stmt contains calls to rayon par_iters.
    // TODO: remove the panics by adding a generic API upstream.
    fn queue_statement<R>(
        &mut self,
        stmt: impl FnOnce(&mut InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint>) -> R,
    ) -> R;
}

// impl this for a single threaded batch verifier.
impl BatchVerifier for InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint> {
    fn queue_statement<R>(
        &mut self,
        stmt: impl FnOnce(&mut InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint>) -> R,
    ) -> R {
        stmt(self)
    }
}
