//! Contains [`IncomingTx`] and [`IncomingTxBuilder`]
use crate::{State, TxState};

/// An incoming transaction that has gone through the preprocessing stage.
pub struct IncomingTx<Tx, TxID, PID> {
    /// The transaction.
    pub(crate) tx: Tx,
    /// The transaction ID.
    pub(crate) tx_id: TxID,
    /// The routing state of the transaction.
    pub(crate) routing_state: TxState<PID>,
}

/// An [`IncomingTx`] builder.
///
// The const generics here are used to restrict what methods can be called.
pub struct IncomingTxBuilder<const RS: bool, const DBS: bool, Tx, TxID, PID> {
    /// The transaction.
    tx: Tx,
    /// The transaction ID.
    tx_id: TxID,
    /// The routing state of the transaction.
    routing_state: Option<TxState<PID>>,

    state_in_db: Option<State>,
}

impl<Tx, TxID, PID> IncomingTxBuilder<false, false, Tx, TxID, PID> {
    /// Crates a new [`IncomingTxBuilder`].
    pub fn new(tx: Tx, tx_id: TxID) -> Self {
        Self {
            tx,
            tx_id,
            routing_state: None,
            state_in_db: None,
        }
    }
}

impl<const DBS: bool, Tx, TxID, PID> IncomingTxBuilder<false, DBS, Tx, TxID, PID> {
    /// Adds the routing state to the builder.
    ///
    /// The routing state is the origin of this transaction from our perspective.
    pub fn with_routing_state(
        self,
        state: TxState<PID>,
    ) -> IncomingTxBuilder<true, DBS, Tx, TxID, PID> {
        IncomingTxBuilder {
            tx: self.tx,
            tx_id: self.tx_id,
            routing_state: Some(state),
            state_in_db: self.state_in_db,
        }
    }
}

impl<const RS: bool, Tx, TxID, PID> IncomingTxBuilder<RS, false, Tx, TxID, PID> {
    /// Adds the database state to the builder.
    ///
    /// If the transaction is not in the DB already then the state should be [`None`].
    pub fn with_state_in_db(
        self,
        state: Option<State>,
    ) -> IncomingTxBuilder<RS, true, Tx, TxID, PID> {
        IncomingTxBuilder {
            tx: self.tx,
            tx_id: self.tx_id,
            routing_state: self.routing_state,
            state_in_db: state,
        }
    }
}

impl<Tx, TxID, PID> IncomingTxBuilder<true, true, Tx, TxID, PID> {
    /// Builds the [`IncomingTx`].
    ///
    /// If this returns [`None`] then the transaction does not need to be given to the dandelion pool
    /// manager.
    pub fn build(self) -> Option<IncomingTx<Tx, TxID, PID>> {
        let routing_state = self.routing_state.unwrap();

        if self.state_in_db == Some(State::Fluff) {
            return None;
        }

        Some(IncomingTx {
            tx: self.tx,
            tx_id: self.tx_id,
            routing_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        IncomingTxBuilder::new(1, 2)
            .with_routing_state(TxState::Stem { from: 3 })
            .with_state_in_db(None)
            .build();

        IncomingTxBuilder::new(1, 2)
            .with_state_in_db(None)
            .with_routing_state(TxState::Stem { from: 3 })
            .build();
    }
}
