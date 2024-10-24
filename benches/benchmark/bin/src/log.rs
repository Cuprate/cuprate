use cfg_if::cfg_if;
use tracing::{info, instrument, Level};
use tracing_subscriber::FmtSubscriber;

/// Initializes the `tracing` logger.
#[instrument]
pub(crate) fn init_logger() {
    const LOG_LEVEL: Level = {
        cfg_if! {
            if #[cfg(feature = "trace")] {
                Level::TRACE
            } else if #[cfg(feature = "debug")] {
                Level::DEBUG
            } else if #[cfg(feature = "warn")] {
                Level::WARN
            } else if #[cfg(feature = "info")] {
                Level::INFO
            } else if #[cfg(feature = "error")] {
                Level::ERROR
            } else {
                Level::INFO
            }
        }
    };

    FmtSubscriber::builder().with_max_level(LOG_LEVEL).init();

    info!("Log level: {LOG_LEVEL}");
}
