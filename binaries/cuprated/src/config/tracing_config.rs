use serde::{Deserialize, Serialize};
use tracing::level_filters::LevelFilter;

/// [`tracing`] config.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct TracingConfig {
    /// The default minimum log level.
    #[serde(with = "level_filter_serde")]
    level: LevelFilter,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            level: LevelFilter::INFO,
        }
    }
}

mod level_filter_serde {
    use std::str::FromStr;

    use serde::{Deserialize, Deserializer, Serializer};
    use tracing::level_filters::LevelFilter;

    #[expect(clippy::trivially_copy_pass_by_ref, reason = "serde")]
    pub fn serialize<S>(level_filter: &LevelFilter, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&level_filter.to_string())
    }

    pub fn deserialize<'de, D>(d: D) -> Result<LevelFilter, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        LevelFilter::from_str(&s).map_err(serde::de::Error::custom)
    }
}
