use std::{cell::RefCell, ops::DerefMut};

use multiexp::BatchVerifier as InternalBatchVerifier;
use rayon::prelude::*;
use thread_local::ThreadLocal;

/// A multithreaded batch verifier.
pub struct MultiThreadedBatchVerifier {
    internal: ThreadLocal<RefCell<InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint>>>,
}

impl MultiThreadedBatchVerifier {
    /// Create a new multithreaded batch verifier,
    pub fn new(numb_threads: usize) -> MultiThreadedBatchVerifier {
        MultiThreadedBatchVerifier {
            internal: ThreadLocal::with_capacity(numb_threads),
        }
    }

    pub fn verify(self) -> bool {
        self.internal
            .into_iter()
            .map(RefCell::into_inner)
            .par_bridge()
            .find_any(|batch_verifier| !batch_verifier.verify_vartime())
            .is_none()
    }
}

impl cuprate_consensus_rules::batch_verifier::BatchVerifier for &'_ MultiThreadedBatchVerifier {
    fn queue_statement<R>(
        &mut self,
        stmt: impl FnOnce(&mut InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint>) -> R,
    ) -> R {
        let mut verifier = self
            .internal
            .get_or(|| RefCell::new(InternalBatchVerifier::new(32)))
            .borrow_mut();

        stmt(verifier.deref_mut())
    }
}
