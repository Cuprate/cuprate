/// Block data for the FCMP tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FcmpLeafBlocks {
    pub hash: [u8; 32],
    pub leaves_in_tree: u64,
}

/// An output for the FCMP tree.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct FcmpOutput {
    pub at_tree_height: u64,
    pub global_output_idx: u64,
    pub leaf_data: FcmpLeafData,
}

/// A leaf in the tree.
///
/// All [`FcmpLeafData`] instances must be valid to put into the tree. i.e. all points should have a
/// non-zero prime order component.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct FcmpLeafData {
    /// The output point.
    ///
    /// This must be as the output is on chain, i.e. should not be torsion cleared. Also should not
    /// be the identity or any of the small order points.
    pub o: [u8; 32],
    /// The outputs amount.
    pub c: FcmpLeafAmount,
}

/// An outputs amount.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum FcmpLeafAmount {
    /// A clear amount.
    Clear(u64),
    /// A commitment.
    ///
    /// This must be as the commitment is on chain, i.e. should not be torsion cleared. Also, should not
    /// be the identity or any of the small order points.
    Commitment([u8; 32]),
}
