#![feature(int_roundings)]
#![feature(io_error_more)]
#![feature(iter_advance_by)]
#![feature(let_chains)]
#![feature(panic_payload_as_str)]
#![feature(anonymous_pipe)]
#![feature(read_buf)]
use std::{fs::{self, File}, io::{self, Error, Stdout}, iter::Once, path::PathBuf, rc::Rc, sync::{LazyLock, Mutex, OnceLock}};

use clap::{crate_version, Arg, Parser};
use anyhow;

mod buffer;
mod handle_event;
mod model;
mod view;
mod parse;
mod logging;
mod themes;
mod syntect_tui;
mod notification;
mod utilities;
mod wrap;
mod suspend;

use logging::{setup_logging, LogError};
use ratatui::{prelude::{Backend, CrosstermBackend}, Terminal};
use tracing::info;
use view::View;
use model::Model;
use handle_event::handle_event;
use buffer::Buffer;

#[cfg(all(feature = "onig", feature = "fancy_regex"))]
compile_error!("feature \"onig\" and feature \"fancy_regex\" cannot be enabled at the same time");

static TERMINAL: OnceLock<Mutex<Terminal<CrosstermBackend<Stdout>>>> = OnceLock::new();

static HELP_TEMPLATE: &'static str = "\
{usage-heading} {usage}

{all-args}

{name} {version} by {author}
";

#[derive(Parser, Debug)]
#[command(author, help_template=HELP_TEMPLATE, version=crate_version!())]
struct Args {
    #[arg(long, help="enable verbose debugging")]
    debug: bool,
    #[arg(long, help="use an alternative logfile path")]
    logfile: Option<PathBuf>,
    #[arg(long, help="do not alter the buffer")]
    readonly: bool,
    #[arg(long, help="visualize whitespace")]
    whitespace: bool,
    files: Option<Vec<String>>
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let _ = setup_logging(&args);
    info!("Launched with {args:?}");

    let buffers = match &args.files {
        Some(files) => read_files(files),
        None => io::Result::Ok(vec![Buffer::empty()]),
    }.log()?;

    let mut terminal = tui::init().log()?;

    tui::install_panic_hook();

    let theme_set = themes::theme_set().log()?;
    let mut model = Model::new(buffers, theme_set, terminal.size().unwrap());
    model.show_whitespace = args.whitespace;

    let mut event_state = handle_event::EventState::default();

    terminal.draw(|frame| model.view(frame))?;
    TERMINAL.set(Mutex::new(terminal)).unwrap();
    while model.running {
        let mut msg = handle_event(&model, &mut event_state)?;
        while msg.is_some() {
            msg = model.update(msg.unwrap());
        }
        TERMINAL.get().unwrap().lock().unwrap().draw(|frame| model.view(frame))?;
    }

    tui::restore()?;
    Ok(())
}

fn read_files(paths: &Vec<String>) -> io::Result<Vec<Buffer>> {
    let mut buffers: Vec<Buffer> = Vec::with_capacity(paths.len());
    for path in paths.iter() {
        let buf = match fs::File::options().read(true).write(true).open(path) {
            Ok(f) => Buffer::new(path.clone(), f, false),
            Err(err) => match err.kind() {
                io::ErrorKind::PermissionDenied => {
                    tracing::debug!("Permission denied opening {path:?}, attempting to open readonly");
                    let f = fs::File::options().read(true).open(path)?;
                    Buffer::new(path.clone(), f, true)
                },
                io::ErrorKind::NotFound => {
                    let mut buf = Buffer::empty();
                    buf.name = Some(path.clone());
                    buf
                },
                _ => return Err(err)
            },
        };

        buffers.push(buf);
    }
    Ok(buffers)
}

mod tui {
    use std::{io::{self, stdout, Stdout}, panic};
    use crossterm::{terminal::*, event::*, ExecutableCommand, QueueableCommand};
    use ratatui::{Terminal, backend::{CrosstermBackend, Backend}};

    pub fn setup() -> io::Result<()> {
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
        stdout().queue(EnableBracketedPaste)?;
        Ok(())
    }

    pub fn init() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
        setup()?;
        Ok(Terminal::new(CrosstermBackend::new(stdout()))?)
    }

    pub fn restore() -> io::Result<()> {
        stdout().execute(PopKeyboardEnhancementFlags)?;
        stdout().execute(DisableMouseCapture)?;
        stdout().execute(LeaveAlternateScreen)?;
        stdout().queue(DisableBracketedPaste)?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn install_panic_hook() {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            stdout().execute(LeaveAlternateScreen).unwrap();
            disable_raw_mode().unwrap();
            tracing::error!("PANIC at {}: {}", info.location().unwrap(), info.payload_as_str().unwrap_or(""));
            original_hook(info);
        }));
    }
}

