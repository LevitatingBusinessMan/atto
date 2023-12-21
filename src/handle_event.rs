use std::time;

use crossterm::event;

use crate::model::{Model, Message};

pub fn handle_event(m: &Model) -> anyhow::Result<Option<Message>> {
    if event::poll(time::Duration::from_millis(16))? {
        match event::read()?  {
            event::Event::Key(key) =>  handle_key(key),
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn handle_key(key: event::KeyEvent) -> anyhow::Result<Option<Message>> {
    if key.kind == crossterm::event::KeyEventKind::Press {
        match key.code {
            event::KeyCode::Right => Ok(Some(Message::NextBuffer)),
            event::KeyCode::Left => Ok(Some(Message::PreviousBuffer)),
            event::KeyCode::Char('q') => Ok(Some(Message::Quit)),
        _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}
