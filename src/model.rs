use std::rc::Rc;

use ratatui::{Frame, layout::{Layout, Constraint, Direction, Rect}, widgets::{Block, Paragraph, Borders, Wrap, Scrollbar, ScrollbarState}, style::{Style, Stylize}, text::Line, Terminal, backend::Backend};

use crate::buffer::Buffer;

pub struct Model {
    /// What buffer is selected
    selected: usize,
    /// What buffers are open
    buffers: Vec<Buffer>,
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
}

/// The top right window
enum UtilityWindow {
    Help,
}

impl Model {

    pub fn new(buffers: Vec<Buffer>) -> Self {
        Model {
            buffers: buffers,
            selected: 0,
            running: true,
            utility: None,
            cursor: (0,0),
            may_scroll: false,
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
                buffer.content.insert(buffer.position, chr as u8);
                buffer.move_right();
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
        }
        None
    }

    fn current_buffer_mut(&mut self) -> &mut Buffer {
        return &mut self.buffers[self.selected];
    }

    fn current_buffer(&self) -> &Buffer {
        return &self.buffers[self.selected];
    }

    pub fn view(&mut self, f: &mut Frame) {
        let main = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(f.size());

        let content_height = self.current_buffer().content.iter().filter(|c| **c == '\n' as u8).count();
        let scrollbar_width = if content_height as u16 > f.size().height {1} else {0};

        let buffer_and_scrollbar = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(scrollbar_width)])
            .split(main[0]);

        let vertical_middle_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.size());

        let utility_area  = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Max(30), Constraint::Length(scrollbar_width)])
            .split(vertical_middle_split[0])[1];

        // Scroll the buffer if the cursor was moved out of view.
        {
            let may_scroll = self.may_scroll;
            let current_buffer = self.current_buffer_mut();
            let (_, cursor_y) = current_buffer.cursor_pos();
            if may_scroll {
                if cursor_y < current_buffer.top as u16 {
                    current_buffer.top = cursor_y as usize;
                } else if cursor_y >= current_buffer.top as u16 + buffer_and_scrollbar[0].height {
                    let diff = cursor_y - (current_buffer.top as u16 + buffer_and_scrollbar[0].height);
                    current_buffer.top += diff as usize + 1;
                }
            }
            self.may_scroll = false;
        }

        let current_buffer = self.current_buffer();

        let (cursor_x, cursor_y) = current_buffer.cursor_pos();

        f.render_widget(
            Paragraph::new(String::from_utf8_lossy(&current_buffer.content))
            .scroll((current_buffer.top as u16,0)),
                buffer_and_scrollbar[0]
        );

        if cursor_y >= self.current_buffer().top as u16 {
            f.set_cursor(cursor_x, cursor_y - self.current_buffer().top as u16);
        }

        let scrollbar = Scrollbar::default();
        let mut scrollbar_state = ScrollbarState::new(content_height).position(self.current_buffer().top);
        
        if scrollbar_width > 0 {
            f.render_stateful_widget(
                scrollbar,
                buffer_and_scrollbar[1],
                &mut scrollbar_state
            );
        }
    
        f.render_widget(
            Paragraph::new(
                Line::styled(
                    std::format!(
                        " {:<} {:>width$} ",
                        "Welcome to Atto! Ctrl-g for help",
                        std::format!("[{}]", self.buffers.iter().map(|b| b.name.clone()).collect::<Vec<String>>().join("|")),
                        width = main[1].width as usize - "Welcome to Atto! Ctrl-g for help".len() - 3
                    ),
                    Style::default()
                    .black()
                    .on_white()
                )
            ),
            main[1]
        );

        match self.utility {
            Some(UtilityWindow::Help) => render_help(f, utility_area),
            None => {},
        }
    }
}

fn render_help(f: &mut Frame, area: Rect) {
    f.render_widget(
        Paragraph::new("Welcome to Atto.\nYou're friendly modern editor.\nHere is a list of shortcuts:\n<enter list of helpful shortcuts>")
        .block(
            Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .border_style(Style::new().blue())
        )
        .wrap(Wrap { trim: false })
    , area);
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
}
