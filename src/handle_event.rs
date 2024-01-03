use std::time;
use anyhow::Ok;
use crossterm::event::{self, KeyModifiers, KeyCode, ModifierKeyCode, Event};
use tracing::debug;

use crate::model::{Model, Message};

pub struct EventState {
    space_down: bool,
    /// This event was pekeed but then wasn't consumed
    peeked_event: Option<Event>,
}

impl Default for EventState {
    fn default() -> Self {
        Self { space_down: false, peeked_event: None }
    }
}

pub fn handle_event(_m: &Model, state: &mut EventState) -> anyhow::Result<Option<Message>> {
    let event = match state.peeked_event.clone() {
        Some(event) => event,
        None => {
            if event::poll(time::Duration::from_millis(16))? {
                event::read()?
            } else {
                return Ok(None)
            }       
        },
    };

    match event  {
        event::Event::Key(key) =>  Ok(handle_key(key, state)),
        event::Event::Mouse(mouse) => Ok(handle_mouse(mouse, state)),
        _ => Ok(None),
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
                _ => None
            }
        } else if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Right => Some(Message::NextBuffer),
                KeyCode::Left => Some(Message::PreviousBuffer),
                KeyCode::Char('q') => Some(Message::Quit),
                KeyCode::Char('g') => Some(Message::OpenHelp),
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

fn handle_mouse(mouse: event::MouseEvent, state: &mut EventState) -> Option<Message> {
    match mouse.kind {
        event::MouseEventKind::ScrollDown | event::MouseEventKind::ScrollUp => {
            let mut scroll: i16 = match mouse.kind {
                event::MouseEventKind::ScrollDown => -1,
                event::MouseEventKind::ScrollUp => 1,
                _ => unreachable!()
            };

            loop {
                use std::result::Result::Ok;
                if let Ok(true) = event::poll(time::Duration::from_millis(0)) {
                    if let Ok(event) = event::read() {
                        state.peeked_event = Some(event.clone());
                        if let Event::Mouse(mouse_event) = event {
                            if mouse_event.kind == event::MouseEventKind::ScrollDown || mouse_event.kind == event::MouseEventKind::ScrollUp {
                                scroll += match mouse_event.kind {
                                    event::MouseEventKind::ScrollDown => -1,
                                    event::MouseEventKind::ScrollUp => 1,
                                    _ => unreachable!(),
                                };
                                state.peeked_event = None;
                            }
                        }
                    }
                }
                break
            }

            debug!("Scroll {scroll}");

            if scroll > 1 {
                Some(Message::ScrollUp(scroll.unsigned_abs()))
            } else {
                Some(Message::ScrollDown(scroll.unsigned_abs()))
            }
        },
        _ => None
    }
}
