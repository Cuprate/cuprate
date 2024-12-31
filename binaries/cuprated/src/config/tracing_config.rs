use serde::{Deserialize, Serialize};
use tracing::level_filters::LevelFilter;

/// [`tracing`] config.
#[derive(Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct TracingConfig {
    pub stdout: StdoutTracingConfig,
    pub file: FileTracingConfig,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct StdoutTracingConfig {
    /// The default minimum log level.
    #[serde(with = "level_filter_serde")]
    pub level: LevelFilter,
}

impl Default for StdoutTracingConfig {
    fn default() -> Self {
        Self {
            level: LevelFilter::INFO,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct FileTracingConfig {
    /// The default minimum log level.
    #[serde(with = "level_filter_serde")]
    pub level: LevelFilter,
    /// The maximum amount of log files to keep, once this number is passed the oldest file
    /// will be deleted.
    pub max_log_files: usize,
}

impl Default for FileTracingConfig {
    fn default() -> Self {
        Self {
            level: LevelFilter::DEBUG,
            max_log_files: 7,
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
