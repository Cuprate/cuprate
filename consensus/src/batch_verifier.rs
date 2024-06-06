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

    pub fn queue_worker(&self) -> QueueWorker<'_> {
        let verifier = self
            .internal
            .get_or(|| RefCell::new(InternalBatchVerifier::new(8)));

        QueueWorker { verifier }
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

pub struct QueueWorker<'a> {
    verifier: &'a RefCell<InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint>>,
}

impl<'a> cuprate_consensus_rules::batch_verifier::BatchVerifier for QueueWorker<'a> {
    fn queue_statement<R>(
        &mut self,
        stmt: impl FnOnce(&mut InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint>) -> R,
    ) -> R {
        let mut verifier = self.verifier.borrow_mut();
        stmt(verifier.deref_mut())
    }
}
