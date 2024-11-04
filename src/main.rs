#![feature(int_roundings)]
#![feature(io_error_more)]
#![feature(iter_advance_by)]
#![feature(let_chains)]
use std::{fs, io, path::PathBuf};

use clap::Parser;
use anyhow;

mod buffer;
mod handle_event;
mod model;
mod view;
mod parse;
mod logging;
mod themes;
mod find;
mod syntect_tui;

use logging::setup_logging;
use tracing::info;
use view::View;
use model::Model;
use handle_event::handle_event;
use buffer::Buffer;

#[cfg(all(feature = "onig", feature = "fancy_regex"))]
compile_error!("feature \"onig\" and feature \"fancy_regex\" cannot be enabled at the same time");

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, help="enable verbose debugging")]
    debug: bool,
    #[arg(long, help="use an alternative logfile path")]
    logfile: Option<PathBuf>,
    files: Option<Vec<String>>
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let _ = setup_logging(&args);
    info!("Launched with {args:?}");

    let buffers = match args.files {
        Some(files) => read_files(files),
        None => io::Result::Ok(vec![Buffer::empty()]),
    }?;

    let mut terminal = tui::init()?;

    tui::install_panic_hook();

    let theme_set = themes::theme_set()?;
    let mut model = Model::new(buffers, theme_set, terminal.size().unwrap());

    let mut event_state = handle_event::EventState::default();

    terminal.draw(|frame| model.view(frame))?;
    while model.running {
        let mut msg = handle_event(&model, &mut event_state)?;
        while msg.is_some() {
            msg = model.update(msg.unwrap());
            terminal.draw(|frame| model.view(frame))?;
        }
    }

    tui::restore()?;
    Ok(())
}

fn read_files(files: Vec<String>) -> io::Result<Vec<Buffer>> {
    files.iter().map(|f| Ok(Buffer::new(
            f.clone(),
            fs::File::options().create(true).read(true).write(true).open(f)?
    ))).collect()
}
mod tui {
    use std::{io::stdout, panic};
    use crossterm::{terminal::*, event::*, ExecutableCommand, QueueableCommand};
    use ratatui::{Terminal, backend::{CrosstermBackend, Backend}};

    pub fn init() -> anyhow::Result<Terminal<impl Backend>> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        stdout().queue(EnableMouseCapture)?;
        // https://docs.rs/crossterm/latest/crossterm/event/struct.KeyboardEnhancementFlags.html
       stdout().queue(PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
        ))?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(terminal)
    }

    pub fn restore() -> anyhow::Result<()> {
        stdout().execute(PopKeyboardEnhancementFlags)?;
        stdout().execute(DisableMouseCapture)?;
        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn install_panic_hook() {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            stdout().execute(LeaveAlternateScreen).unwrap();
            disable_raw_mode().unwrap();
            original_hook(panic_info);
        }));
    }
}
