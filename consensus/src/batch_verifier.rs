use std::{cell::RefCell, ops::DerefMut};

use multiexp::BatchVerifier as InternalBatchVerifier;
use rayon::prelude::*;
use thread_local::ThreadLocal;

use crate::ConsensusError;

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

    pub fn queue_statement<R>(
        &self,
        stmt: impl FnOnce(
            &mut InternalBatchVerifier<(), dalek_ff_group::EdwardsPoint>,
        ) -> Result<R, ConsensusError>,
    ) -> Result<R, ConsensusError> {
        let verifier_cell = self
            .internal
            .get_or(|| RefCell::new(InternalBatchVerifier::new(8)));
        // TODO: this is not ok as a rayon par_iter could be called in stmt.
        stmt(verifier_cell.borrow_mut().deref_mut())
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
