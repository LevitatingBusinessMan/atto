use std::{fs, io};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn setup_logging() -> io::Result<()> {
    let file = fs::File::create("atto.log")?;
    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(file)
        .with_target(false)
        .with_ansi(true);
    tracing_subscriber::registry().with(file_subscriber).init();
    Ok(())
}