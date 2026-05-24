//! Logging
//!
//! `cuprated` log filtering settings and related functionality.
use std::ops::BitAnd;
use std::{
    fmt::{Display, Formatter},
    sync::OnceLock,
};
use tracing::{
    instrument::WithSubscriber, level_filters::LevelFilter, subscriber::Interest, Metadata,
};
use tracing_appender::{
    non_blocking::{NonBlocking, WorkerGuard},
    rolling::Rotation,
};
use tracing_subscriber::{
    filter::Filtered,
    fmt::{
        self,
        format::{DefaultFields, Format},
        Layer as FmtLayer,
    },
    layer::{Context, Filter, Layered, SubscriberExt},
    reload::{Handle, Layer as ReloadLayer},
    util::SubscriberInitExt,
    Layer, Registry,
};

use cuprate_helper::fs::logs_path;

use crate::config::Config;

/// A [`OnceLock`] which holds the [`Handle`] to update the file logging output.
///
/// Initialized in [`init_logging`].
static FILE_WRITER_FILTER_HANDLE: OnceLock<Handle<CupratedTracingFilter, Registry>> =
    OnceLock::new();

/// A [`OnceLock`] which holds the [`Handle`] to update the stdout logging output.
///
/// Initialized in [`init_logging`].
#[expect(clippy::type_complexity)] // factoring out isn't going to help readability.
static STDOUT_FILTER_HANDLE: OnceLock<
    Handle<
        CupratedTracingFilter,
        Layered<
            Filtered<
                FmtLayer<Registry, DefaultFields, Format, NonBlocking>,
                ReloadLayer<CupratedTracingFilter, Registry>,
                Registry,
            >,
            Registry,
            Registry,
        >,
    >,
> = OnceLock::new();

/// The [`Filter`] used to alter cuprated's log output.
#[derive(Debug)]
pub struct CupratedTracingFilter {
    pub level: LevelFilter,
}

// Custom display behavior for command output.
impl Display for CupratedTracingFilter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filter")
            .field("minimum_level", &self.level.to_string())
            .finish()
    }
}

impl<S> Filter<S> for CupratedTracingFilter {
    fn enabled(&self, meta: &Metadata<'_>, cx: &Context<'_, S>) -> bool {
        Filter::<S>::enabled(&self.level, meta, cx)
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        Filter::<S>::callsite_enabled(&self.level, meta)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(self.level)
    }
}

/// Create the non-blocking file appender and its flush [`WorkerGuard`].
///
/// The returned [`WorkerGuard`] must be held for as long as logs should be written to the file;
/// dropping it flushes any buffered lines to disk. Losing the guard (for example via
/// [`mem::forget`](std::mem::forget)) silently drops the final buffered lines on shutdown — exactly
/// the lines that matter for crash/shutdown post-mortems.
fn create_file_appender(config: &Config) -> (NonBlocking, WorkerGuard) {
    let appender_config = &config.tracing.file;
    tracing_appender::non_blocking(
        tracing_appender::rolling::Builder::new()
            .rotation(Rotation::DAILY)
            .max_log_files(appender_config.max_log_files)
            .build(logs_path(&config.fs.fast_data_directory, config.network()))
            .unwrap(),
    )
}

/// Initialize [`tracing`] for logging to stdout and to a file.
///
/// Returns the file appender's [`WorkerGuard`]. The caller **must** hold it for the lifetime of the
/// program and only drop it during shutdown, so the non-blocking appender flushes its buffered log
/// lines to the file before the process exits.
#[must_use = "dropping the WorkerGuard stops the file appender from flushing on shutdown"]
pub fn init_logging(config: &Config) -> WorkerGuard {
    // initialize the stdout filter, set `STDOUT_FILTER_HANDLE` and create the layer.
    let (stdout_filter, stdout_handle) = ReloadLayer::new(CupratedTracingFilter {
        level: config.tracing.stdout.level,
    });

    STDOUT_FILTER_HANDLE.set(stdout_handle).unwrap();

    let stdout_layer = FmtLayer::default()
        .with_target(false)
        .with_filter(stdout_filter);

    // create the tracing appender.
    let appender_config = &config.tracing.file;
    let (appender, guard) = create_file_appender(config);

    // initialize the appender filter, set `FILE_WRITER_FILTER_HANDLE` and create the layer.
    let (appender_filter, appender_handle) = ReloadLayer::new(CupratedTracingFilter {
        level: appender_config.level,
    });
    FILE_WRITER_FILTER_HANDLE.set(appender_handle).unwrap();

    let appender_layer = fmt::layer()
        .with_target(false)
        .with_ansi(false)
        .with_writer(appender)
        .with_filter(appender_filter);

    // initialize tracing with the 2 layers.
    tracing_subscriber::registry()
        .with(appender_layer)
        .with(stdout_layer)
        .init();

    // Hand the flush guard back to the caller, which must hold it until shutdown.
    guard
}

/// Modify the stdout [`CupratedTracingFilter`].
///
/// Must only be called after [`init_logging`].
pub fn modify_stdout_output(f: impl FnOnce(&mut CupratedTracingFilter)) {
    STDOUT_FILTER_HANDLE.get().unwrap().modify(f).unwrap();
}

/// Modify the file appender [`CupratedTracingFilter`].
///
/// Must only be called after [`init_logging`].
pub fn modify_file_output(f: impl FnOnce(&mut CupratedTracingFilter)) {
    FILE_WRITER_FILTER_HANDLE.get().unwrap().modify(f).unwrap();
}

/// Prints some text using [`eprintln`], with [`nu_ansi_term::Color::Red`] applied.
pub fn eprintln_red(s: &str) {
    eprintln!("{}", nu_ansi_term::Color::Red.bold().paint(s));
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;

    use cuprate_helper::fs::logs_path;

    use super::create_file_appender;
    use crate::config::Config;

    /// Concatenate every file in `dir` into one string.
    fn read_all_logs(dir: &std::path::Path) -> String {
        let mut out = String::new();
        for entry in std::fs::read_dir(dir).expect("log directory should exist after a write") {
            let path = entry.expect("readable dir entry").path();
            if path.is_file() {
                out.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
        out
    }

    /// Dropping the [`WorkerGuard`](tracing_appender::non_blocking::WorkerGuard) returned by
    /// [`create_file_appender`] must flush buffered lines to disk. This is the exact property the
    /// shutdown path relies on, and the one the old `mem::forget(guard)` destroyed.
    #[test]
    fn file_appender_flushes_on_guard_drop() {
        let tmp = tempdir().expect("create temp dir");
        let mut config = Config::default();
        config.fs.fast_data_directory = tmp.path().to_path_buf();

        let canary = "shutdown-flush-canary-line";
        let (mut appender, guard) = create_file_appender(&config);
        writeln!(appender, "{canary}").expect("write to non-blocking appender");

        // Drop signals the worker thread to flush remaining lines before returning.
        drop(guard);

        let logs_dir = logs_path(&config.fs.fast_data_directory, config.network());
        let contents = read_all_logs(&logs_dir);
        assert!(
            contents.contains(canary),
            "buffered log line must be flushed to disk once the guard is dropped; got: {contents:?}"
        );
    }
}
