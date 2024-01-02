#![feature(int_roundings)]
#![feature(io_error_more)]
use std::{io, fs, path::{Path, self}};

use clap::Parser;
use anyhow;

mod buffer;
mod handle_event;
mod model;
mod view;

use view::View;
use model::Model;
use handle_event::handle_event;
use buffer::Buffer;

#[derive(Parser)]
struct Args {
    files: Option<Vec<String>>
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let buffers = match args.files {
        Some(files) => read_files(files),
        None => io::Result::Ok(vec![Buffer::empty()]),
    }?;

    let mut terminal = tui::init()?;

    tui::install_panic_hook();

    let mut model = Model::new(buffers);

    let mut event_state = handle_event::EventState::default();

    while model.running {
        terminal.draw(|frame| model.view(frame))?;
        let mut msg = handle_event(&model, &mut event_state)?;
        while msg.is_some() {
            msg = model.update(msg.unwrap());
        }
    }

    tui::restore()?;

    Ok(())
}

fn read_files(files: Vec<String>) -> io::Result<Vec<Buffer>> {
    files.iter().map(|f| Ok(Buffer::new(
            f.clone(),
            fs::read_to_string(f)?
    ))).collect()
}
mod tui {
    use std::{io::stdout, panic};
    use crossterm::{terminal::{EnterAlternateScreen, enable_raw_mode, LeaveAlternateScreen, disable_raw_mode}, ExecutableCommand, Command, QueueableCommand, event::KeyboardEnhancementFlags};
    use ratatui::{Terminal, backend::{CrosstermBackend, Backend}};

    pub fn init() -> anyhow::Result<Terminal<impl Backend>> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        stdout().queue(crossterm::event::EnableMouseCapture)?;
        // https://docs.rs/crossterm/latest/crossterm/event/struct.KeyboardEnhancementFlags.html
        stdout().queue(crossterm::event::PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES))?;
        stdout().queue(crossterm::event::PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES))?;
        stdout().queue(crossterm::event::PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES))?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(terminal)
    }

    pub fn restore() -> anyhow::Result<()> {
        stdout().execute(LeaveAlternateScreen)?;
        stdout().execute(crossterm::event::PopKeyboardEnhancementFlags)?;
        stdout().execute(crossterm::event::DisableMouseCapture)?;
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
