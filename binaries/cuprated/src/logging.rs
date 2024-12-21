use std::{
    fmt::{Display, Formatter},
    mem::forget,
    sync::OnceLock,
};

use tracing::{
    instrument::WithSubscriber, level_filters::LevelFilter, subscriber::Interest, Metadata,
};
use tracing_appender::{non_blocking::NonBlocking, rolling::Rotation};
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
pub struct CupratedTracingFilter {
    pub level: LevelFilter,
}

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

/// Initialize [`tracing`] for logging to stdout and to a file.
pub fn init_logging(config: &Config) {
    // initialize the stdout filter, set `STDOUT_FILTER_HANDLE` and create the layer.
    let (stdout_filter, stdout_handle) = ReloadLayer::new(CupratedTracingFilter {
        level: config.tracing.stdout.level,
    });

    drop(STDOUT_FILTER_HANDLE.set(stdout_handle));

    let stdout_layer = FmtLayer::default()
        .with_target(false)
        .with_filter(stdout_filter);

    // create the tracing appender.
    let appender_config = &config.tracing.file;
    let (appender, guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::Builder::new()
            .rotation(Rotation::DAILY)
            .max_log_files(appender_config.max_log_files)
            .build(logs_path(&config.fs.data_directory, config.network()))
            .unwrap(),
    );

    // TODO: drop this when we shutdown.
    forget(guard);

    // initialize the appender filter, set `FILE_WRITER_FILTER_HANDLE` and create the layer.
    let (appender_filter, appender_handle) = ReloadLayer::new(CupratedTracingFilter {
        level: appender_config.level,
    });
    drop(FILE_WRITER_FILTER_HANDLE.set(appender_handle));

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
