use std::{rc::Rc, collections::HashMap, cell::RefCell};

use ratatui::{Frame, layout::{Layout, Constraint, Direction, Rect}, widgets::{Block, Paragraph, Borders, Wrap, Scrollbar, ScrollbarState}, style::{Style, Stylize}, text::Line, Terminal, backend::Backend};
use syntect::{highlighting::{ThemeSet, Theme}, parsing::SyntaxSet};
use tracing::debug;

use crate::{buffer::Buffer, parse::{ParseCache, self}};

pub struct Model {
    /// What buffer is selected
    pub selected: usize,
    /// What buffers are open
    pub buffers: Vec<Buffer>,
    /// If we should close the application
    pub running: bool,
    /// The utility window
    pub utility: Option<UtilityWindow>,
    /// Where should the cursor be drawn
    pub cursor: (u16, u16),
    /// Tell the view it may have to scroll
    /// the buffer because the cursor might've moved 
    /// out of view; 
    pub may_scroll: bool,
    pub parse_caches: HashMap<String, Rc<RefCell<ParseCache>>>,
    pub theme_set: ThemeSet,
    pub syntax_set: SyntaxSet,
    pub theme: String,
}

/// The top right window
pub enum UtilityWindow {
    Help,
    Find(crate::find::FindModel),
}

impl Model {
    pub fn new<'a>(mut buffers: Vec<Buffer>, theme_set: ThemeSet) -> Model {
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
            cursor: (0,0),
            may_scroll: false,
            parse_caches,
            theme_set,
            syntax_set,
            theme: "dracula".to_owned(),
        }
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {

        debug!("{msg:?}");

        match &mut self.utility {
            Some(UtilityWindow::Find(find)) => {
                match msg {
                    Message::Escape | Message::Quit | Message::Find(_) => {},
                    _ => {
                        return find.update(msg)
                    }
                }
            },
            _ => {}
        }

        match msg {
            Message::NextBuffer => self.selected = (self.selected + 1) % self.buffers.len(),
            Message::PreviousBuffer => self.selected = (self.selected + self.buffers.len() - 1) % self.buffers.len(),
            Message::Quit => self.running = false,
            Message::ScrollDown => self.current_buffer_mut().top += 1,
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
            Message::Backspace => {
                let cur = self.current_buffer_mut();
                if cur.position > 0 {
                    cur.content.remove(cur.position-1);
                    return Some(Message::MoveLeft);
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
            }
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
    Backspace,
    JumpWordLeft,
    JumpWordRight,
    GotoStartOfLine,
    GotoEndOfLine,
    Enter,
}
