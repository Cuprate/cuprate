use crate::config::Config;
use cuprate_helper::fs::logs_path;
use std::mem::forget;
use std::sync::OnceLock;
use tracing::instrument::WithSubscriber;
use tracing::level_filters::LevelFilter;
use tracing::subscriber::Interest;
use tracing::Metadata;
use tracing_appender::non_blocking::NonBlocking;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::filter::Filtered;
use tracing_subscriber::fmt::format::{DefaultFields, Format};
use tracing_subscriber::layer::{Context, Layered, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{
    fmt::Layer as FmtLayer,
    layer::Filter,
    reload::{Handle, Layer as ReloadLayer},
    Layer,
};
use tracing_subscriber::{reload, Registry};

static FILE_WRITER_FILTER_HANDLE: OnceLock<Handle<CupratedTracingFilter, Registry>> =
    OnceLock::new();

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

pub struct CupratedTracingFilter {
    pub level: LevelFilter,
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

pub fn init_logging(config: &Config) {
    use tracing_subscriber::{fmt, Layer};

    let (stdout_filter, stdout_handle) = reload::Layer::new(CupratedTracingFilter {
        level: config.tracing.stdout.level,
    });
    drop(STDOUT_FILTER_HANDLE.set(stdout_handle));

    let stdout_layer = fmt::Layer::default()
        .with_target(false)
        .with_filter(stdout_filter);

    let appender_config = &config.tracing.file;
    let (appender, guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::Builder::new()
            .rotation(Rotation::DAILY)
            .max_log_files(appender_config.max_log_files)
            .build(logs_path(&config.fs.data_directory, config.network()))
            .unwrap(),
    );

    forget(guard);

    let (appender_filter, appender_handle) = reload::Layer::new(CupratedTracingFilter {
        level: appender_config.level,
    });
    drop(FILE_WRITER_FILTER_HANDLE.set(appender_handle));

    let appender_layer = fmt::layer()
        .with_target(false)
        .with_ansi(false)
        .with_writer(appender)
        .with_filter(appender_filter);

    tracing_subscriber::registry()
        .with(appender_layer)
        .with(stdout_layer)
        .init();
}

pub fn modify_stdout_output(f: impl FnOnce(&mut CupratedTracingFilter)) {
    STDOUT_FILTER_HANDLE.get().unwrap().modify(f).unwrap();
}

pub fn modify_file_output(f: impl FnOnce(&mut CupratedTracingFilter)) {
    FILE_WRITER_FILTER_HANDLE.get().unwrap().modify(f).unwrap();
}
