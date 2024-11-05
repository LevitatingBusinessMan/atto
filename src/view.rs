//! For rendering the model
use std::{cell::RefCell, rc::Rc};

use color_eyre::owo_colors::OwoColorize;
use ratatui::{layout::{Alignment, Constraint, Direction, Layout}, style::{Style, Stylize}, text::{Line, Text}, widgets::{Clear, Paragraph, Scrollbar, ScrollbarState}, Frame};
use syntect::{util::LinesWithEndings, highlighting::{Highlighter, Theme}, parsing::SyntaxSet};

use crate::{model::Model, parse::{parse_from, ParseCache}, utilities::{Utility}};
use crate::buffer::Buffer;
use crate::utilities::UtilityWindow;

/// files over this size might be handled differently (like not having a scrollbar)
pub static LARGE_FILE_LIMIT: usize = 1_000_000;

pub trait View {
    fn view(&mut self, f: &mut Frame);
}

impl View for Model {
    #[tracing::instrument(skip_all, level="trace")]
    fn view(&mut self, f: &mut Frame) {
        // split between status bar and rest
        let main = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(f.area());

        let large_file = self.current_buffer().content.len() > LARGE_FILE_LIMIT;
        let content_height = if large_file { usize::MAX } else { self.current_buffer().content.chars().filter(|c| *c == '\n').count() };
        let scrollbar_width = if content_height as u16 > f.area().height {1} else {0};

        let buffer_and_scrollbar = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(scrollbar_width)])
            .split(main[0]);

        let vertical_middle_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());

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
                tracing::error!("{:?}", e);
                // TODO cover tabs here
                Paragraph::new(current_buffer.content.as_str()).scroll((current_buffer.top as u16,0))
            },
        };

        f.render_widget(
            buffer_widget,
            buffer_and_scrollbar[0]
        );

        if cursor_y >= self.current_buffer().top as u16 {
            f.set_cursor_position((cursor_x, cursor_y - self.current_buffer().top as u16));
        }

        let scrollbar = Scrollbar::default();
            let mut scrollbar_state = if large_file { ScrollbarState::new(1) } else {
            ScrollbarState::new(content_height.saturating_sub(f.area().height as usize))
            .position(self.current_buffer().top)
        };
        
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
                        std::format!("[{}]", self.buffers.iter().map(
                            |b| b.name.clone() +
                            (if b.dirty().unwrap_or_else(|e| {tracing::error!("{:?}", e); true}) { "+" } else { "" })
                        ).collect::<Vec<String>>().join("|")),
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
            Some(UtilityWindow::Help(help)) => help.view(&self, f, utility_area),
            Some(UtilityWindow::Find(find)) => find.view(&self, f, utility_area),
            Some(UtilityWindow::Confirm(confirm)) => confirm.view(&self, f, utility_area),
            Some(UtilityWindow::Developer(developer)) => developer.view(&self, f, utility_area),
            None => {},
        }

        // render notification
        if let Some(notification) = &self.notification {
            let buffer = buffer_and_scrollbar[0];
            let wrapped_content = textwrap::fill(&notification.content, buffer.width as usize);
            let height = wrapped_content.lines().count();
            let mut area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(height as u16)])
                .split(buffer)[1];
            // notifiations that take up no more than a single line
            // are aligned to the right and only the text is colorized
            let alignment = if height > 1 { Alignment::Left } else { Alignment::Right };
            if height < 2 {
                let width = wrapped_content.chars().count();
                area = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(0), Constraint::Length(width as u16)])
                    .split(area)[1];
            } else {
                f.render_widget(Clear, area);
            }
            let widget = Text::raw(wrapped_content)
                .style(notification.style)
                .alignment(alignment);
            f.render_widget(widget, area);
        }
    }
}

fn highlight<'a>(buffer: &'a Buffer, height: usize, cache: Rc<RefCell<ParseCache>>, syntax_set: &SyntaxSet, theme: &Theme) -> anyhow::Result<Vec<Line<'a>>> {
    let lines = LinesWithEndings::from(&buffer.content);
    let hl = Highlighter::new(theme);
    let syntax = buffer.syntax.as_ref().unwrap_or(syntax_set.find_syntax_plain_text());
    parse_from(buffer.top, lines, height, &mut cache.borrow_mut(), &hl, syntax, &syntax_set)
}
