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
    if let KeyCode::Char('j') | KeyCode::Char('f') = key.code {
        if let KeyCode::Char(char) = key.code {
            debug!("{char}");
            debug!("{char}");
            match key.kind {
                event::KeyEventKind::Press => {
                    debug!("movement_key_down = {char}");
                    state.movement_key_down = Some(char);
                },
                event::KeyEventKind::Release => {
                    if let Some(current) = state.movement_key_down {
                        if current == char {
                    debug!("movement_key_down = None");
                            state.movement_key_down = None;
                        }
                    }
                },
                event::KeyEventKind::Repeat => {},
            }
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
                KeyCode::Char('f') => {
                    if state.space_down || key.modifiers.contains(KeyModifiers::CONTROL) {
                        Some(Message::JumpWordLeft)
                    } else {
                        Some(Message::MoveLeft)
                    }
                },
                KeyCode::Char('a') => Some(Message::GotoStartOfLine),
                KeyCode::Char('e') => Some(Message::GotoEndOfLine),
                // Reverse word jumping
                KeyCode::Char(' ') => match state.movement_key_down {
                    Some('j') => Some(Message::JumpWordRight),
                    Some('f') => Some(Message::JumpWordLeft),
                    _ => None,
                },
                _ => None
            }
        } else if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Right => Some(Message::NextBuffer),
                KeyCode::Left => Some(Message::PreviousBuffer),
                KeyCode::Char('q') => Some(Message::Quit),
                KeyCode::Char('s') => Some(Message::Save),
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
                KeyCode::Delete => Some(Message::Delete),
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
