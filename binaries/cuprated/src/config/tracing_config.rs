use serde::{Deserialize, Serialize};
use tracing::level_filters::LevelFilter;

use super::macros::config_struct;

config_struct! {
    /// [`tracing`] config.
    #[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct TracingConfig {
        /// Redact sensitive data from logs.
        ///
        /// When enabled, sensitive values such as transaction hashes are
        /// replaced with `[scrubbed]` in logs, so a shared log file does not
        /// leak data that could deanonymise you or your peers.
        ///
        /// Disable this only if you need the raw values, e.g. for debugging.
        ///
        /// Type         | boolean
        /// Valid values | true, false
        pub redact: bool,

        #[child = true]
        /// Configuration for cuprated's stdout logging system.
        pub stdout: StdoutTracingConfig,

        #[child = true]
        /// Configuration for cuprated's file logging system.
        pub file: FileTracingConfig,
    }
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            redact: true,
            stdout: StdoutTracingConfig::default(),
            file: FileTracingConfig::default(),
        }
    }
}

config_struct! {
    #[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct StdoutTracingConfig {
        /// The minimum log level for stdout.
        ///
        /// Levels below this one will not be shown.
        /// "error" is the highest level only showing errors,
        /// "trace" is the lowest showing as much as possible.
        ///
        /// Type         | Level
        /// Valid values | "error", "warn", "info", "debug", "trace"
        ##[serde(with = "level_filter_serde")]
        pub level: LevelFilter,
    }
}

impl Default for StdoutTracingConfig {
    fn default() -> Self {
        Self {
            level: LevelFilter::INFO,
        }
    }
}

config_struct! {
    #[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct FileTracingConfig {
        /// The minimum log level for file logs.
        ///
        /// Levels below this one will not be shown.
        /// "error" is the highest level only showing errors,
        /// "trace" is the lowest showing as much as possible.
        ///
        /// Type         | Level
        /// Valid values | "error", "warn", "info", "debug", "trace"
        ##[serde(with = "level_filter_serde")]
        pub level: LevelFilter,

        /// The maximum amount of log files to keep.
        ///
        /// Once this number is passed the oldest file will be deleted.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 0, 7, 200
        pub max_log_files: usize,
    }
}

impl Default for FileTracingConfig {
    fn default() -> Self {
        Self {
            level: LevelFilter::DEBUG,
            max_log_files: 7,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn redact_defaults_to_true() {
        assert!(TracingConfig::default().redact);
    }

    #[test]
    fn redact_can_be_disabled() {
        let config: TracingConfig = toml::from_str("redact = false").unwrap();
        assert!(!config.redact);
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
