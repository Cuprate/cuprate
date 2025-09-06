use std::cell::RefCell;

use monero_oxide::ringct::bulletproofs::BatchVerifier as InternalBatchVerifier;
use rayon::prelude::*;
use thread_local::ThreadLocal;

use cuprate_consensus_rules::batch_verifier::BatchVerifier;

/// A multithreaded batch verifier.
pub struct MultiThreadedBatchVerifier {
    internal: ThreadLocal<RefCell<InternalBatchVerifier>>,
}

impl MultiThreadedBatchVerifier {
    /// Create a new multithreaded batch verifier,
    pub fn new(numb_threads: usize) -> Self {
        Self {
            internal: ThreadLocal::with_capacity(numb_threads),
        }
    }

    pub fn verify(self) -> bool {
        self.internal
            .into_iter()
            .map(RefCell::into_inner)
            .par_bridge()
            .try_for_each(|batch_verifier| {
                if batch_verifier.verify() {
                    Ok(())
                } else {
                    Err(())
                }
            })
            .is_ok()
    }
}

impl BatchVerifier for &'_ MultiThreadedBatchVerifier {
    fn queue_statement<R>(&mut self, stmt: impl FnOnce(&mut InternalBatchVerifier) -> R) -> R {
        let mut verifier = self
            .internal
            .get_or(|| RefCell::new(InternalBatchVerifier::new()))
            .borrow_mut();

        stmt(&mut verifier)
    }
}
