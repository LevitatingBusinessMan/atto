//! For rendering the model

use std::os::linux::raw::stat;

use color_eyre::owo_colors::OwoColorize;
use ratatui::{layout::{Alignment, Constraint, Direction, Layout}, style::{Style, Stylize}, text::Line, widgets::{Clear, Paragraph, Scrollbar, ScrollbarState}, Frame};
use syntect::{util::LinesWithEndings, highlighting::{Highlighter, Theme}, parsing::SyntaxSet};
use tracing::trace;
use ratatui::prelude::*;

use crate::{model::Model, parse::parse_from, utilities::{Utility}};
use crate::buffer::Buffer;
use crate::utilities::UtilityWindow;

/// files over this size might be handled differently (like not having a scrollbar)
pub static LARGE_FILE_LIMIT: usize = 1_000_000;

pub struct AttoLayout {
    pub buffer: Rect,
    pub scrollbar: Option<Rect>,
    /// the status bar
    pub status: Rect,
    /// whole area
    pub all: Rect,
    /// upper half
    pub upper: Rect,
    /// lower half
    pub lower: Rect,
    pub utility: Rect,
}

impl Model {
    pub fn layout(&self) -> AttoLayout {
        let all = Rect { x: 0, y: 0, width: self.viewport.width, height: self.viewport.height };

        // split between status bar and rest
        let status_bar_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(all);

        let content_height = self.current_buffer().linestarts.len() - 1;
        let with_scrollbar = content_height as u16 > self.viewport.height;

        let buffer_and_scrollbar = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(status_bar_split[0]);

        let buffer = if with_scrollbar {
            buffer_and_scrollbar[0]
        } else {
            status_bar_split[0]
        };

        let scrollbar = if with_scrollbar {
            Some(buffer_and_scrollbar[1])
        } else {
            None
        };

        let vertical_middle_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(all);

        let utility  = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Max(30), Constraint::Length(if with_scrollbar {1} else {0})])
            .split(vertical_middle_split[0])[1];

        AttoLayout {
            all,
            buffer,
            scrollbar,
            utility,
            status: status_bar_split[1],
            upper: vertical_middle_split[1],
            lower: vertical_middle_split[0],
        }
    }

    #[tracing::instrument(skip_all, level="trace")]
    pub fn view(&self, f: &mut Frame) {
        let layout = self.layout();

        let content_height = self.current_buffer().linestarts.len() - 1;
        let scrollbar_width = if content_height as u16 > self.viewport.height {1} else {0};

        let current_buffer = self.current_buffer();

        let buffer_widget = match highlight(current_buffer, layout.buffer.height as usize, &self.syntax_set, self.theme(), self.show_whitespace) {
            Ok(tokens) => Paragraph::new(tokens),
            Err(e) => {
                tracing::error!("{:?}", e);
                // TODO unless we can cover stuff like tabs and showing whitespace here (and wordwrapping)
                // we really should rely on our own parse function
                // and this should be a hard error
                Paragraph::new(current_buffer.content.as_str()).scroll((current_buffer.top as u16,0))
            },
        };

        f.render_widget(
            buffer_widget,
            layout.buffer
        );

        // if in view, display cursor
        // TODO fix scrolling up and cursor stickking at the bottom
        if self.current_buffer().cursor.y >= self.current_buffer().top {
            f.set_cursor_position((self.current_buffer().cursor.x as u16, self.current_buffer().cursor.y as u16 - self.current_buffer().top as u16));
        }

        if let Some(scrollbar_area) = layout.scrollbar {
            let scrollbar = Scrollbar::default();
            let mut scrollbar_state = ScrollbarState::new(content_height.saturating_sub(f.area().height as usize))
                .position(self.current_buffer().top);
            f.render_stateful_widget(
                scrollbar,
                scrollbar_area,
                &mut scrollbar_state
            );
        }



        f.render_widget(
            Paragraph::new(
                Line::styled(
                    std::format!(
                        " {:<} {:>width$} ",
                        "Welcome to Atto! Ctrl-h for help",
                        std::format!("{} ({}/{}) at b{} {}{} {}/{}",
                            self.current_buffer().syntax.clone().map_or("plain".to_string(), |s| s.name.to_lowercase()),
                            self.current_buffer().cursor.x + 1,
                            self.current_buffer().cursor.y + 1,
                            self.current_buffer().position,
                            self.current_buffer().name.clone().unwrap_or("?".to_string()),
                            if self.current_buffer().dirty().unwrap() { "+" } else { "" },
                            self.selected+1, self.buffers.len(),
                        ),
                        width = self.viewport.width as usize - "Welcome to Atto! Ctrl-h for help".len() - 3
                    ),
                    Style::default()
                    .black()
                    .on_white()
                )
            ),
            layout.status
        );

        match &self.utility {
            Some(UtilityWindow::Help(help)) => help.view(f, layout.utility),
            Some(UtilityWindow::Find(find)) => find.view(f, layout.utility),
            Some(UtilityWindow::Confirm(confirm)) => confirm.view(f, layout.utility),
            Some(UtilityWindow::Developer(developer)) => developer.view(f, layout.utility),
            Some(UtilityWindow::Shell(shell)) => shell.view(f, layout.utility),
            Some(UtilityWindow::SaveAs(save_as)) => save_as.view(f, layout.utility),
            None => {},
        }

        // render notification
        if let Some(notification) = &self.notification {
            let wrapped_content = textwrap::fill(&notification.content, layout.buffer.width as usize);
            let height = wrapped_content.lines().count();
            let mut area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(height as u16)])
                .split(layout.buffer)[1];
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
            let widget = Paragraph::new(wrapped_content)
                .style(notification.style)
                .scroll((height.saturating_sub(area.height as usize) as u16,0))
                .alignment(alignment);
            f.render_widget(widget, area);
        }
    }
}

/// Parse and highlight a buffer
fn highlight<'a>(buffer: &'a Buffer, height: usize, syntax_set: &SyntaxSet, theme: &Theme, show_whitespace: bool) -> anyhow::Result<Vec<Line<'a>>> {
    let lines = LinesWithEndings::from(&buffer.content);
    let hl = Highlighter::new(theme);
    let syntax = buffer.syntax.as_ref().unwrap_or(syntax_set.find_syntax_plain_text());
    parse_from(buffer.top, lines, height, &mut buffer.parse_cache.borrow_mut(), &hl, syntax, &syntax_set, show_whitespace)
}
