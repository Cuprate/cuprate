//---------------------------------------------------------------------------------------------------- Backend
#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Backend {
    #[default]
    Heed,
    Redb,
}
