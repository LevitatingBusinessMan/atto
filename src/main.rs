#![feature(int_roundings)]
use clap::Parser;
use anyhow;
use std::{fs, io::{stdout, self}, rc::Rc, ops::Deref};
use ratatui::{prelude::*, widgets::*};

mod buffer;

use buffer::Buffer;

#[derive(Parser)]
struct Args {
    files: Option<Vec<String>>
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let buffers = match args.files {
        Some(files) => files.iter().map(|f| io::Result::Ok(Buffer {
            name: f.clone(),
            content: fs::read(f)?
        })).collect(),
        None => io::Result::Ok(vec![Buffer::empty()]),
    }?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    
    terminal.clear()?;

    let buffers = Rc::new(buffers);
    let selected = 0;

    loop {
        terminal.draw(|frame| {
            let main = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Max(1000), Constraint::Length(1)])
                .split(frame.size());

            frame.render_widget(
                Paragraph::new(String::from_utf8_lossy(&buffers[selected].content))
                    .block(Block::default()
                    .title(buffers[selected].name.clone())
                    .borders(
                            Borders::TOP | Borders::RIGHT | Borders::LEFT
                    )),
                    main[0]
            );
        
            frame.render_widget(
                Paragraph::new(
                    Line::styled(
                        std::format!(
                            " {:<} {:>width$} ",
                            "Welcome to Atto!",
                            std::format!("[{}]", buffers.iter().map(|b| b.name.clone()).collect::<Vec<String>>().join("|")),
                            width = main[1].width as usize - "Welcome to Atto!".len() - 3
                        ),
                        Style::default()
                        .add_modifier(Modifier::REVERSED)
                    )
                ),
                main[1]
            )

        })?;
    }

    Ok(())
}

fn ui(frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new("Hello World!")
            .block(Block::default().title("Greeting").borders(Borders::ALL)),
        frame.size(),
    );
}
