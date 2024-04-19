/// The current dandelion++ state.
enum State {
    /// Fluff state, in this state we are diffusing stem transactions to all peers.
    Fluff,
    /// Stem state, in this state we are stemming stem transactions to a single outbound peer.
    Stem,
}
