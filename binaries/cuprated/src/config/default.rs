use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum DefaultOrCustom<T> {
    Default,
    #[serde(untagged)]
    Custom(T),
}
