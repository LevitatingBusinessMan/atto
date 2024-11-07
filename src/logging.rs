use std::{fs, io};
use dirs;

use tracing::{info, level_filters::LevelFilter, Level};
use tracing_subscriber::{fmt::{format::FmtSpan, writer::MakeWriterExt}, layer::SubscriberExt, Layer, Registry};

pub fn setup_logging(args: &crate::Args) -> io::Result<()> {
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

    let level = if args.debug || cfg!(debug_assertions) { Level::TRACE } else { Level::INFO };

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .with_writer(file)
        .with_target(true)
        .with_ansi(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(LevelFilter::from_level(level));

    let subscriber = Registry::default()
        .with(fmt_layer);

    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("log level is {level:?}");

    Ok(())
}

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
