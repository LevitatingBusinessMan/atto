use std::{fs, io};
use dirs;

use tracing::{info, Level};
use tracing_subscriber::{fmt::{format::FmtSpan, writer::MakeWriterExt}, layer::SubscriberExt, util::SubscriberInitExt};

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

    let level = if args.debug || cfg!(debug_assertions) { Level::TRACE } else { Level::WARN };

    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_writer(file)
        .with_target(true)
        .with_ansi(true)
        .with_max_level(level)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    Ok(())
}
