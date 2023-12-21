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
    loop {
        terminal.draw(|frame| {
            let main = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Max(1000), Constraint::Length(1)])
                .split(frame.size());

            let buf_layout = Layout::new(
                Direction::Horizontal,
                buffers.iter().map(|b| Constraint::Percentage(100_u16.div_floor(buffers.len() as u16)))
            ).split(main[0]);
    
            for (i, buffer) in buffers.deref().iter().enumerate() {
                frame.render_widget(
                    Paragraph::new(String::from_utf8_lossy(&buffer.content))
                        .block(Block::default().title(buffer.name.clone()).borders(
                            if i == 0 {
                                Borders::TOP | Borders::RIGHT | Borders::LEFT
                            } else {
                                Borders::TOP | Borders::RIGHT
                            }
                        )),
                        buf_layout[i]
                );
            }

            frame.render_widget(
                Paragraph::new(
                    Line::styled(
                        std::format!("{:width$}", "Welcome to Atto!", width = main[1].width as usize),
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
