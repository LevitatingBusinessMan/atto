use std::{rc::Rc, collections::HashMap, cell::RefCell};

use ratatui::{Frame, layout::{Layout, Constraint, Direction, Rect}, widgets::{Block, Paragraph, Borders, Wrap, Scrollbar, ScrollbarState}, style::{Style, Stylize}, text::Line, Terminal, backend::Backend};

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
}

/// The top right window
pub enum UtilityWindow {
    Help,
}

impl Model {

    pub fn new<'a>(buffers: Vec<Buffer>) -> Model {
        let parse_caches = (|| {
            let mut map = HashMap::new();
            for buf in &buffers {
                map.insert(buf.name.clone(), Rc::new(RefCell::new(ParseCache::new())));
            }
            map
        })();
        Model {
            buffers: buffers,
            selected: 0,
            running: true,
            utility: None,
            cursor: (0,0),
            may_scroll: false,
            parse_caches,
        }
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::NextBuffer => self.selected = (self.selected + 1) % self.buffers.len(),
            Message::PreviousBuffer => self.selected = (self.selected + self.buffers.len() - 1) % self.buffers.len(),
            Message::Quit => self.running = false,
            Message::ScrollDown => self.current_buffer_mut().top += 1,
            Message::ScrollUp => self.current_buffer_mut().top = self.current_buffer_mut().top.checked_sub(1).unwrap_or_default(),
            Message::OpenHelp => self.utility = Some(UtilityWindow::Help),
            Message::Escape => {
                if self.utility.is_some() {
                    self.utility = None;
                }
            },
            Message::InsertChar(chr) => {
                let buffer = self.current_buffer_mut();
                buffer.content.insert(buffer.position, chr);
                buffer.move_right();
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
            Message::JumpWordRight => {
                self.current_buffer_mut().move_word_right();
                self.may_scroll = true;
            },
            Message::GotoStartOfLine => self.current_buffer_mut().goto_start_of_line(),
            Message::GotoEndOfLine => self.current_buffer_mut().goto_end_of_line(),
            Message::Enter => return Some(Message::InsertChar('\n')),
        }
        None
    }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        return &mut self.buffers[self.selected];
    }

    pub fn current_buffer(&self) -> &Buffer {
        return &self.buffers[self.selected];
    }
}
pub enum Message {
    NextBuffer,
    PreviousBuffer,
    Quit,
    ScrollDown,
    ScrollUp,
    OpenHelp,
    Escape,
    InsertChar(char),
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Backspace,
    JumpWordRight,
    GotoStartOfLine,
    GotoEndOfLine,
    Enter,
}
