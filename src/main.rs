#![feature(int_roundings)]
use std::{io, fs};

use clap::Parser;
use anyhow;

mod buffer;
mod handle_event;
mod model;

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


    while model.running {
        terminal.draw(|frame| model.view(frame))?;
        let mut msg = handle_event(&model)?;
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
        fs::read(f)?
    ))).collect()
}
mod tui {
    use std::{io::stdout, panic};
    use crossterm::{terminal::{EnterAlternateScreen, enable_raw_mode, LeaveAlternateScreen, disable_raw_mode}, ExecutableCommand, Command, QueueableCommand};
    use ratatui::{Terminal, backend::{CrosstermBackend, Backend}};

    pub fn init() -> anyhow::Result<Terminal<impl Backend>> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        stdout().queue(crossterm::event::EnableMouseCapture)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(terminal)
    }

    pub fn restore() -> anyhow::Result<()> {
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
