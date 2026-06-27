use std::{fs, io};
use dirs;

use tracing::{Level, debug, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt::format::FmtSpan, layer::SubscriberExt};
use unicode_segmentation::GraphemeIncomplete;
use std::ffi::CStr;

pub fn setup_logging(args: &crate::Args) -> io::Result<()> {
    let default_level = if args.debug || cfg!(debug_assertions) { Level::TRACE } else { Level::INFO };
    let env = EnvFilter::builder()
            .with_default_directive(default_level.into())
            .from_env().map_err(|_| io::Error::other("env filter failed"))?;

    let syslog = {
        static IDENTITY: &'static CStr = c"atto";
        let (options, facility) = Default::default();
        let writer = syslog_tracing::Syslog::new(IDENTITY, options, facility)
            .ok_or(io::Error::other("failed to create syslog writer"))?;
        tracing_subscriber::fmt::layer()
                .without_time()
                .with_target(false)
                .with_ansi(false)
                .with_writer(writer)
    };
    
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

    let file_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .with_writer(file)
        .with_target(true)
        .with_ansi(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE);
        //.with_filter(LevelFilter::from_level(level));

    let use_log_file = false;
        
    let subscriber = Registry::default()
        .with(env)
        .with(syslog)
        .with(use_log_file.then_some(file_layer));

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
