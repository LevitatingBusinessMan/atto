use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Instant};

use ratatui::{layout::Size, style::{Color, Style}};
use syntect::{highlighting::{ThemeSet, Theme}, parsing::SyntaxSet};
use tracing::debug;
use tracing_subscriber::filter::combinator::Not;

use crate::buffer::Buffer;
use crate::parse::ParseCache;
use crate::notification::Notification;

pub struct Model {
    /// What buffer is selected
    pub selected: usize,
    /// What buffers are open
    pub buffers: Vec<Buffer>,
    /// If we should close the application
    pub running: bool,
    /// The utility window
    pub utility: Option<UtilityWindow>,
    /// Tell the view it may have to scroll
    /// the buffer because the cursor might've moved 
    /// out of view; 
    pub may_scroll: bool,
    pub parse_caches: HashMap<String, Rc<RefCell<ParseCache>>>,
    pub theme_set: ThemeSet,
    pub syntax_set: SyntaxSet,
    pub theme: String,
    pub viewport: Size,
    pub notification: Option<Notification>,
}

/// The top right window
pub enum UtilityWindow {
    Help,
    Find(crate::find::FindModel),
}

impl Model {
    pub fn new<'a>(mut buffers: Vec<Buffer>, theme_set: ThemeSet, viewport: Size) -> Model {
        let parse_caches = (|| {
            let mut map = HashMap::new();
            for buf in &buffers {
                map.insert(buf.name.clone(), Rc::new(RefCell::new(ParseCache::new())));
            }
            map
        })();

        let syntax_set = SyntaxSet::load_defaults_newlines();
        for buffer in &mut buffers {
            buffer.find_syntax(&syntax_set);
        }
        Model {
            buffers: buffers,
            selected: 0,
            running: true,
            utility: None,
            may_scroll: false,
            parse_caches,
            theme_set,
            syntax_set,
            theme: "dracula".to_owned(),
            viewport,
            notification: None,
        }
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {

        debug!("{msg:?}");

        match &mut self.utility {
            Some(UtilityWindow::Find(find)) => {
                match msg {
                    Message::Escape | Message::Quit | Message::Find(_) => {},
                    _ => {
                        // this logic is off
                        // the utility should not intercept (and remove) all
                        // messages by default
                        // it should intercept, and then return, remove or replace it
                        return find.update(msg)
                    }
                }
            },
            _ => {}
        }

        // remove notification if elapsed
        // (the handling of this makes it so that if the user does not somehow create a message)
        // the notification will hang around
        if let Some(notification) = &self.notification {
            if notification.expired() {
                self.notification = None;
            }
        }

        match msg {
            Message::NextBuffer => self.selected = (self.selected + 1) % self.buffers.len(),
            Message::PreviousBuffer => self.selected = (self.selected + self.buffers.len() - 1) % self.buffers.len(),
            Message::Quit => self.running = false,
            Message::ScrollDown => {
                if (self.current_buffer().content.lines().count() - self.viewport.height as usize) > self.current_buffer_mut().top {
                 self.current_buffer_mut().top += 1;
                }
            },
            Message::ScrollUp => self.current_buffer_mut().top = self.current_buffer_mut().top.checked_sub(1).unwrap_or_default(),
            Message::OpenHelp => self.utility = Some(UtilityWindow::Help),
            Message::OpenFind => self.utility = Some(UtilityWindow::Find(crate::find::FindModel::new())),
            Message::Escape => {
                if self.utility.is_some() {
                    self.utility = None;
                }
            },
            Message::InsertChar(chr) => {
                self.current_buffer_mut().insert(chr);
                self.may_scroll = true;
            },
            Message::MoveLeft => {
                self.current_buffer_mut().move_left();
                self.may_scroll = true;
            },
            Message::MoveRight => {
                self.current_buffer_mut().move_right();
                self.may_scroll = true;
            },
            Message::MoveUp => {
                self.current_buffer_mut().move_up();
                self.may_scroll = true;
            },
            Message::MoveDown => {
                self.current_buffer_mut().move_down();
                self.may_scroll = true;
            },
            Message::PageUp => {
                let height = self.viewport.height as usize;
                self.current_buffer_mut().page_up(height);
                // self.may_scroll = true;
            },
            Message::PageDown => {
                let height = self.viewport.height as usize;
                self.current_buffer_mut().page_down(height);
                // self.may_scroll = true;
            },
            Message::Backspace => {
                let cur = self.current_buffer_mut();
                if cur.position > 0 {
                    cur.content.remove(cur.position-1);
                    return Some(Message::MoveLeft);
                }
            },
            Message::Delete => {
                let cur = self.current_buffer_mut();
                if cur.position < cur.content.len() {
                    cur.content.remove(cur.position);
                }
            },
            Message::JumpWordLeft => {
                self.current_buffer_mut().move_word_left();
                self.may_scroll = true;
            },
            Message::JumpWordRight => {
                self.current_buffer_mut().move_word_right();
                self.may_scroll = true;
            },
            Message::GotoStartOfLine => self.current_buffer_mut().goto_start_of_line(),
            Message::GotoEndOfLine => self.current_buffer_mut().goto_end_of_line(),
            Message::Enter => return Some(Message::InsertChar('\n')),
            Message::Find(query) => {
                self.current_buffer_mut().find(query);
                self.may_scroll = true;
            },
            Message::Save => {
                if let Err(e) =  self.current_buffer_mut().save() {
                    tracing::error!("{:?}", e);
                    return Some(Message::Notification(
                        format!("Error writing file: {e:?}"),
                        Style::new().bg(Color::Green).fg(Color::Black)
                    ));
                } else {
                    return Some(Message::Notification(
                        String::from("SAVED"),
                        Style::new().bg(Color::Green).fg(Color::Black)
                    ));
                }
            },
            Message::Resize(x, y) => {
                self.viewport = (x,y).into();
            },
            Message::MouseLeft(x, y) => {
                self.current_buffer_mut().set_viewport_cursor_pos(x, y);
            },
            Message::Notification(content, style) => {
                self.notification = Some(Notification::new(content, style));
            },
        }        
        None
    }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        return &mut self.buffers[self.selected];
    }

    pub fn current_buffer(&self) -> &Buffer {
        return &self.buffers[self.selected];
    }

    pub fn theme(&self) -> &Theme {
        return &self.theme_set.themes[&self.theme]
    }
}

#[derive(Debug)]
pub enum Message {
    NextBuffer,
    PreviousBuffer,
    Quit,
    ScrollDown,
    ScrollUp,
    OpenHelp,
    OpenFind,
    Find(String),
    Escape,
    InsertChar(char),
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Backspace,
    Delete,
    JumpWordLeft,
    JumpWordRight,
    GotoStartOfLine,
    GotoEndOfLine,
    Enter,
    Save,
    Resize(u16, u16),
    MouseLeft(u16, u16),
    Notification(String, Style),
}
