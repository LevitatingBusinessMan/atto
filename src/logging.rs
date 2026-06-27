use std::{fs, io, os::fd::{AsRawFd, FromRawFd, IntoRawFd}};
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

    let syslog_layer = {
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

    let use_log_file = true;

    let (pipe_read, pipe_write) = nix::unistd::pipe()?;
    nix::unistd::dup2(pipe_read.as_raw_fd(), 100)?;
    let fd_file = unsafe { fs::File::from_raw_fd(pipe_write.into_raw_fd()) };

    let pipe_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .with_writer(fd_file)
        .with_target(true)
        .with_ansi(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE);
    
    let subscriber = Registry::default()
        .with(env)
        .with(syslog_layer)
        .with(use_log_file.then_some(file_layer))
        .with(pipe_layer);

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
