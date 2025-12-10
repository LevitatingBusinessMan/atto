use std::{fs, io};
use dirs;

use tracing::{Level, debug, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt::format::FmtSpan, layer::SubscriberExt};
use unicode_segmentation::GraphemeIncomplete;

pub fn setup_logging(args: &crate::Args) -> io::Result<()> {
    let default_level = if args.debug || cfg!(debug_assertions) { Level::TRACE } else { Level::INFO };
    let env = EnvFilter::builder()
            .with_default_directive(default_level.into())
            .from_env().map_err(|_| io::Error::other("env filter failed"))?;

    let file = fs::File::options()
        .write(true)
        .append(true)
        .create(true)
        .open(
            args.logfile.clone().unwrap_or(
                dirs::cache_dir().ok_or_else(|| io::Error::other("failed to find cache dir"))?
                .join("atto.log")
            )
        )?;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .with_writer(file)
        .with_target(true)
        .with_ansi(true)
        .with_span_events(FmtSpan::CLOSE);
        //.with_filter(LevelFilter::from_level(level));

    let subscriber = Registry::default()
        .with(env)
        .with(fmt_layer);

    let _ = tracing::subscriber::set_global_default(subscriber);

    Ok(())
}

/// Trait for logging different kinds of errors
pub trait LogError {
    /// If this result is an error, log it as such
    fn log(self) -> Self;
}

impl<T> LogError for io::Result<T> {
    fn log(self) -> Self {
        if let Err(err) = &self {
            tracing::error!("{err:?}");
        }
        self
    }
}

impl<T> LogError for anyhow::Result<T> {
    fn log(self) -> Self {
        if let Err(err) = &self {
            tracing::error!("{err:?}");
        }
        self
    }
}

impl<T> LogError for core::result::Result<T, GraphemeIncomplete> {
    fn log(self) -> Self {
        if let Err(err) = &self {
            tracing::error!("{err:?}");
        }
        self
    }
}
