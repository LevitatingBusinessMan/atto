use std::time;

use anyhow::Ok;
use crossterm::event::{self, KeyModifiers, KeyCode};

use crate::model::{Model, Message};

pub fn handle_event(_m: &Model) -> anyhow::Result<Option<Message>> {
    if event::poll(time::Duration::from_millis(16))? {
        match event::read()?  {
            event::Event::Key(key) =>  Ok(handle_key(key)),
            event::Event::Mouse(mouse) => Ok(handle_mouse(mouse)),
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn handle_key(key: event::KeyEvent) -> Option<Message> {
    if key.kind == crossterm::event::KeyEventKind::Press {
        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Right => Some(Message::NextBuffer),
                KeyCode::Left => Some(Message::PreviousBuffer),
                KeyCode::Char('q') => Some(Message::Quit),
                KeyCode::Char('g') => Some(Message::OpenHelp),
                _ => None,
            }
        } else if key.modifiers == KeyModifiers::ALT {
            match key.code {
                // Originally, p for uP
                KeyCode::Char('o') => Some(Message::MoveUp),
                KeyCode::Char('n') => Some(Message::MoveDown),
                KeyCode::Char('j') => Some(Message::MoveRight),
                KeyCode::Char('f') => Some(Message::MoveLeft),
                _ => None
            }
        } else {
            match key.code {
                KeyCode::Esc => Some(Message::Escape),
                KeyCode::Char(char) => Some(Message::InsertChar(char)),
                KeyCode::Enter => Some(Message::InsertChar('\n')),
                KeyCode::Left => Some(Message::MoveLeft),
                KeyCode::Right => Some(Message::MoveRight),
                KeyCode::Up => Some(Message::MoveUp),
                KeyCode::Down => Some(Message::MoveDown),
                KeyCode::Backspace => Some(Message::Backspace),
                _ => None
            }
        }
    } else {
        None
    }
}

fn handle_mouse(mouse: event::MouseEvent) -> Option<Message> {
    match mouse.kind {
        event::MouseEventKind::ScrollDown => Some(Message::ScrollDown),
        event::MouseEventKind::ScrollUp => Some(Message::ScrollUp),
        _ => None
    }
}
