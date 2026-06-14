use serde::{Deserialize, Serialize};
use tracing::level_filters::LevelFilter;

use super::macros::config_struct;

config_struct! {
    /// [`tracing`] config.
    #[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct TracingConfig {
        #[child = true]
        /// Configuration for cuprated's stdout logging system.
        pub stdout: StdoutTracingConfig,

        #[child = true]
        /// Configuration for cuprated's file logging system.
        pub file: FileTracingConfig,

        /// Whether to redact sensitive data (transaction hashes, peer and onion
        /// addresses) from logs.
        ///
        /// When `true` (the default), such data is replaced with `[scrubbed]` (IP
        /// addresses are partially redacted). Do not disable this on a public or
        /// shared node, as the log file becomes a deanonymization target.
        ///
        /// Type         | boolean
        /// Valid values | true, false
        pub redact: bool,

        /// Allow disabling log redaction even when RPC is publicly reachable.
        ///
        /// ⚠️ WARNING ⚠️
        /// -------------
        /// Log redaction should almost never be disabled on a public node.
        /// If redaction is disabled (see `redact`) while RPC is bound to a
        /// non-local address, cuprated will panic, unless this setting is
        /// set to `true`.
        ///
        /// Type         | boolean
        /// Valid values | true, false
        pub i_know_what_im_doing_allow_unredacted_public_logs: bool,
    }
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            stdout: StdoutTracingConfig::default(),
            file: FileTracingConfig::default(),
            redact: true,
            i_know_what_im_doing_allow_unredacted_public_logs: false,
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
