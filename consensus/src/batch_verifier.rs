use std::cell::UnsafeCell;

use multiexp::BatchVerifier as InternalBatchVerifier;
use rayon::prelude::*;
use thread_local::ThreadLocal;

use crate::ConsensusError;

/// A multi threaded batch verifier.
pub struct MultiThreadedBatchVerifier {
    internal: ThreadLocal<UnsafeCell<InternalBatchVerifier<usize, dalek_ff_group::EdwardsPoint>>>,
}

impl MultiThreadedBatchVerifier {
    /// Create a new multithreaded batch verifier,
    pub fn new(numb_threads: usize) -> MultiThreadedBatchVerifier {
        MultiThreadedBatchVerifier {
            internal: ThreadLocal::with_capacity(numb_threads),
        }
    }

    pub fn queue_statement(
        &self,
        stmt: impl FnOnce(
            &mut InternalBatchVerifier<usize, dalek_ff_group::EdwardsPoint>,
        ) -> Result<(), ConsensusError>,
    ) -> Result<(), ConsensusError> {
        let verifier_cell = self
            .internal
            .get_or(|| UnsafeCell::new(InternalBatchVerifier::new(0)));
        // SAFETY: This is safe for 2 reasons:
        //  1. each thread gets a different batch verifier.
        //  2. only this function `queue_statement` will get the inner batch verifier, it's private.
        //
        // TODO: it's probably ok to just use RefCell
        stmt(unsafe { &mut *verifier_cell.get() })
    }

    pub fn verify(self) -> bool {
        self.internal
            .into_iter()
            .map(UnsafeCell::into_inner)
            .par_bridge()
            .find_any(|batch_verifer| !batch_verifer.verify_vartime())
            .is_none()
    }
}
