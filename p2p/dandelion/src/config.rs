use std::{
    ops::{Mul, Neg},
    time::Duration,
};

/// When calculating the embargo timeout using the formula: `(-k*(k-1)*hop)/(2*log(1-ep))`
///
/// (1 - ep) is the probability that a transaction travels for `k` hops before a nodes embargo timeout fires, this constant is (1 - ep).
const EMBARGO_FULL_TRAVEL_PROBABILITY: f64 = 0.90;

/// The graph type to use for dandelion routing, the dandelion paper recommends [Graph::FourRegular].
///
/// The decision between line graphs and 4-regular graphs depend on the priorities of the system, if
/// linkability of transactions is a first order concern then line graphs may be better, however 4-regular graphs
/// can give constant-order privacy benefits against adversaries with knowledge of the graph.
///
/// See appendix C of the dandelion++ paper.
#[derive(Default, Debug, Copy, Clone)]
pub enum Graph {
    /// Line graph.
    ///
    /// When this is selected one peer will be chosen from the outbound peers each epoch to route transactions
    /// to.
    ///
    /// In general this is not recommend over [`Graph::FourRegular`] but may be better for certain systems.
    Line,
    /// Quasi-4-Regular.
    ///
    /// When this is selected two peers will be chosen from the outbound peers each epoch, each stem transaction
    /// received will then be sent to one of these two peers. Transactions from the same node will always go to the
    /// same peer.
    #[default]
    FourRegular,
}

/// The config used to initialize dandelion.
///
/// One notable missing item from the config is `Tbase` AKA the timeout parameter to prevent black hole
/// attacks. This is removed from the config for simplicity, `Tbase` is calculated using the formula provided
/// in the D++ paper:
///
///  `(-k*(k-1)*hop)/(2*log(1-ep))`
///
/// Where `k` is calculated from the fluff probability, `hop` is `time_between_hop` and `ep` is fixed at `0.1`.
///
#[derive(Debug, Clone, Copy)]
pub struct DandelionConfig {
    /// The time it takes for a stem transaction to pass through a node, including network latency.
    ///
    /// It's better to be safe and put a slightly higher value than lower.
    pub time_between_hop: Duration,
    /// The duration of an epoch.
    pub epoch_duration: Duration,
    /// q in the dandelion paper, this is the probability that a node will be in the fluff state for
    /// a certain epoch.
    ///
    /// The dandelion paper recommends to make this value small, but the smaller this value, the higher
    /// the broadcast latency.
    ///
    /// It is recommended for this value to be <= `0.2`, this value *MUST* be between 0 and 1 (not equal to either).
    pub fluff_probability: f64,
    /// The graph type.
    pub graph: Graph,
}

impl DandelionConfig {
    /// Returns the number of outbound peers to use to stem transactions.
    ///
    /// This value depends on the [`Graph`] chosen.
    pub fn number_of_stems(&self) -> usize {
        match self.graph {
            Graph::Line => 1,
            Graph::FourRegular => 2,
        }
    }

    /// Returns the average embargo timeout, `Tbase` in the dandelion++ paper.
    ///
    /// This is the average embargo timeout _only including this node_ with k nodes also putting an embargo timeout
    /// using the exponential distrobution, the average until one of them fluffs is `Tbase / k`.
    pub fn average_embargo_timeout(&self) -> Duration {
        // we set k equal to the expected stem length with this fluff probability.
        let k = self.expected_stem_length();
        let time_between_hop = self.time_between_hop.as_secs_f64();

        Duration::from_secs_f64(
            // (-k*(k-1)*hop)/(2*ln(1-ep))
            ((k.neg() * (k - 1.0) * time_between_hop)
                / EMBARGO_FULL_TRAVEL_PROBABILITY.ln().mul(2.0))
            .ceil(),
        )
    }

    /// Returns the expected length of a stem.
    pub fn expected_stem_length(&self) -> f64 {
        self.fluff_probability.recip()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        f64::consts::E,
        ops::{Mul, Neg},
        time::Duration,
    };

    use proptest::{prop_assert, proptest};

    use super::*;

    #[test]
    fn monerod_average_embargo_timeout() {
        let cfg = DandelionConfig {
            time_between_hop: Duration::from_millis(175),
            epoch_duration: Default::default(),
            fluff_probability: 0.125,
            graph: Default::default(),
        };

        assert_eq!(cfg.average_embargo_timeout(), Duration::from_secs(47));
    }

    proptest! {
        #[test]
        fn embargo_full_travel_probablity_correct(time_between_hop in 1_u64..1_000_000, fluff_probability in 0.000001..1.0) {
            let cfg = DandelionConfig {
                time_between_hop: Duration::from_millis(time_between_hop),
                epoch_duration: Default::default(),
                fluff_probability,
                graph: Default::default(),
            };

            // assert that the `average_embargo_timeout` is high enough that the probability of `k` nodes
            // not diffusing before expected diffusion is greater than or equal to `EMBARGO_FULL_TRAVEL_PROBABLY`
            //
            // using the formula from in appendix B.5
            let k = cfg.expected_stem_length();
            let time_between_hop = cfg.time_between_hop.as_secs_f64();

            let average_embargo_timeout = cfg.average_embargo_timeout().as_secs_f64();

            let probability =
                E.powf((k.neg() * (k - 1.0) * time_between_hop) / average_embargo_timeout.mul(2.0));

            prop_assert!(probability >= EMBARGO_FULL_TRAVEL_PROBABILITY, "probability = {probability}, average_embargo_timeout = {average_embargo_timeout}");
        }
    }
}
