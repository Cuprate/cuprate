use multiexp::BatchVerifier as InternalBatchVerifier;

pub trait BatchVerifier {
    fn queue_statement<R>(
        &mut self,
        stmt: impl FnOnce(&mut InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint>) -> R,
    ) -> R;
}

impl BatchVerifier for InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint> {
    fn queue_statement<R>(
        &mut self,
        stmt: impl FnOnce(&mut InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint>) -> R,
    ) -> R {
        stmt(self)
    }
}
