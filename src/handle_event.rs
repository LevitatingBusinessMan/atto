use std::time;

use crossterm::event::{self, ModifierKeyCode, KeyModifiers, KeyCode};

use crate::model::{Model, Message};

pub fn handle_event(m: &Model) -> anyhow::Result<Option<Message>> {
    if event::poll(time::Duration::from_millis(16))? {
        match event::read()?  {
            event::Event::Key(key) =>  handle_key(key),
            event::Event::Mouse(mouse) => handle_mouse(mouse),
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn handle_key(key: event::KeyEvent) -> anyhow::Result<Option<Message>> {
    if key.kind == crossterm::event::KeyEventKind::Press {
        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Right => Ok(Some(Message::NextBuffer)),
                KeyCode::Left => Ok(Some(Message::PreviousBuffer)),
                KeyCode::Char('q') => Ok(Some(Message::Quit)),
                KeyCode::Char('g') => Ok(Some(Message::OpenHelp)),
                _ => Ok(None),
            }
        } else {
            match key.code {
                KeyCode::Esc => Ok(Some(Message::Escape)),
                KeyCode::Char(char) => Ok(Some(Message::InsertChar(char))),
                KeyCode::Enter => Ok(Some(Message::InsertChar('\n'))),
                KeyCode::Left => Ok(Some(Message::MoveLeft)),
                KeyCode::Right => Ok(Some(Message::MoveRight)),
                KeyCode::Up => Ok(Some(Message::MoveUp)),
                KeyCode::Down => Ok(Some(Message::MoveDown)),
                _ => Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

fn handle_mouse(mouse: event::MouseEvent) -> anyhow::Result<Option<Message>> {
    match mouse.kind {
        event::MouseEventKind::ScrollDown => Ok(Some(Message::ScrollDown)),
        event::MouseEventKind::ScrollUp => Ok(Some(Message::ScrollUp)),
        _ => Ok(None)
    }
}
