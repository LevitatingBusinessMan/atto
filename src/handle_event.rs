use std::time;

use anyhow::Ok;
use crossterm::event::{self, KeyModifiers, KeyCode, ModifierKeyCode};
use tracing::debug;

use crate::model::{Model, Message};

pub struct EventState {
    /// For word jumping
    space_down: bool,
    /// For reverse word jumping with space
    movement_key_down: Option<char>,
}

impl Default for EventState {
    fn default() -> Self {
        Self { space_down: false, movement_key_down: None }
    }
}

pub fn handle_event(_m: &Model, state: &mut EventState) -> anyhow::Result<Option<Message>> {
    if event::poll(time::Duration::from_millis(0))? {
        match event::read()?  {
            event::Event::Key(key) =>  Ok(handle_key(key, state)),
            event::Event::Mouse(mouse) => Ok(handle_mouse(mouse)),
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn handle_key(key: event::KeyEvent, state: &mut EventState) -> Option<Message> {

    debug!("{key:?}");

    // Space as a modifier key
    if key.code == KeyCode::Char(' ') {
        if state.space_down && key.kind == crossterm::event::KeyEventKind::Release {
            state.space_down = false;
        }
        if !state.space_down && key.kind == crossterm::event::KeyEventKind::Press {
            state.space_down = true;
        }
    }

    // If a movement key is held (so space can jump in that direction)
    if key.code == KeyCode::Char('j') {
        if let Some('j') = state.movement_key_down && key.kind == crossterm::event::KeyEventKind::Release {
            state.movement_key_down = None;
        } else if key.kind == crossterm::event::KeyEventKind::Press {
            state.movement_key_down = Some('j');
        }
    }

    if key.kind == crossterm::event::KeyEventKind::Press || key.kind == crossterm::event::KeyEventKind::Repeat {
        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Char('i') => Some(Message::MoveUp),
                KeyCode::Char('n') => Some(Message::MoveDown),
                KeyCode::Char('j') => {
                    if state.space_down || key.modifiers.contains(KeyModifiers::CONTROL) {
                        Some(Message::JumpWordRight)
                    } else {
                        Some(Message::MoveRight)
                    }
                },
                KeyCode::Char('f') => Some(Message::MoveLeft),
                KeyCode::Char('a') => Some(Message::GotoStartOfLine),
                KeyCode::Char('e') => Some(Message::GotoEndOfLine),
                // Reverse word jumping
                KeyCode::Char(' ') => {
                    if let Some('j') = state.movement_key_down {
                        Some(Message::JumpWordRight)
                    } else {
                        None
                    }
                }
                _ => None
            }
        } else if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Right => Some(Message::NextBuffer),
                KeyCode::Left => Some(Message::PreviousBuffer),
                KeyCode::Char('q') => Some(Message::Quit),
                KeyCode::Char('g') => Some(Message::OpenHelp),
                KeyCode::Char('f') => Some(Message::OpenFind),
                _ => None,
            }
        } else {
            match key.code {
                KeyCode::Esc => Some(Message::Escape),
                KeyCode::Char(char) => Some(Message::InsertChar(char)),
                KeyCode::Enter => Some(Message::Enter),
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
