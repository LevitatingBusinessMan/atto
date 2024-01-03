//! For rendering the model
use std::{io::BufRead, cell::RefCell, rc::Rc};

use anyhow::anyhow;
use ratatui::{Frame, layout::{Direction, Constraint, Layout, Rect}, widgets::{Paragraph, Scrollbar, ScrollbarState, Wrap, Block, Borders, Clear}, text::{Line, Span}, style::{Style, Stylize}};
use syntect::{util::LinesWithEndings, highlighting::{Highlighter, ThemeSet, Theme}, parsing::SyntaxSet};
use tracing::debug;

use crate::{model::{Model, UtilityWindow, self}, parse::{parse_from, ParseCache}};
use crate::buffer::Buffer;

pub trait View {
    fn view(&mut self, f: &mut Frame);
}

impl View for Model {
    fn view(&mut self, f: &mut Frame) {
        let main = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(f.size());

        let content_height = self.current_buffer().content.chars().filter(|c| *c == '\n').count();
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

        let cache = self.parse_caches.get(&current_buffer.name).unwrap().clone();

        let buffer_widget = match highlight(current_buffer, buffer_and_scrollbar[0].height as usize, cache, &self.syntax_set, self.theme()) {
            Ok(tokens) => Paragraph::new(tokens),
            Err(e) => {
                Paragraph::new(current_buffer.content.as_str()).scroll((current_buffer.top as u16,0))
            },
        };

        f.render_widget(
            buffer_widget,
            buffer_and_scrollbar[0]
        );

        if cursor_y >= self.current_buffer().top as u16 {
            f.set_cursor(cursor_x, cursor_y - self.current_buffer().top as u16);
        }

        let scrollbar = Scrollbar::default();
        let mut scrollbar_state = ScrollbarState::new(content_height.saturating_sub(f.size().height as usize))
        .position(self.current_buffer().top);
        
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

        match &self.utility {
            Some(UtilityWindow::Help) => render_help(f, utility_area),
            Some(UtilityWindow::Find(find)) => find.render(f, utility_area),
            None => {},
        }
    }
}

fn highlight<'a>(buffer: &'a Buffer, height: usize, cache: Rc<RefCell<ParseCache>>, syntax_set: &SyntaxSet, theme: &Theme) -> anyhow::Result<Vec<Line<'a>>> {
    let lines = LinesWithEndings::from(&buffer.content);
    let hl = Highlighter::new(theme);
    let syntax = buffer.syntax.as_ref().unwrap_or(syntax_set.find_syntax_plain_text());
    parse_from(buffer.top, lines, height, &mut cache.borrow_mut(), &hl, syntax, &syntax_set)
}

fn render_help(f: &mut Frame, area: Rect) {

    f.render_widget(Clear, area);

    f.render_widget(
        Paragraph::new(
r"Welcome to Atto!
Here is a list of keybinds:
C-c Copy
C-x Cut
C-v Paste
C-a Select All
A-a Start
A-e End
A-j Right
A-i Up
A-f Left
A-n Down
C-f Find
C-e Command
"
)
        .block(
            Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .border_style(Style::new().blue())
        )
        .wrap(Wrap { trim: false })
    , area);
}

