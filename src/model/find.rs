//! The find utility and widget

use ratatui::{Frame, layout::Rect, widgets::{Paragraph, Clear, Block, Borders}, style::{Style, Stylize}};

use super::Message;
pub struct FindUtility {
    pub query: String,
    pub position: usize,
}

impl FindUtility {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            position: 0,
        }
    }
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let widget = Paragraph::new(format!("> {}", self.query))
            .block(
                Block::default()
                .title("Find")
                .borders(Borders::ALL)
                .border_style(Style::new().blue())
            );
        f.render_widget(Clear, area);
        f.render_widget(widget, area);
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::ScrollDown => todo!(),
            Message::ScrollUp => todo!(),
            Message::InsertChar(chr) => {
                self.query.insert(self.position, chr);
                self.position += 1;
                None
            },
            Message::MoveLeft => todo!(),
            Message::MoveRight => todo!(),
            Message::MoveUp => todo!(),
            Message::MoveDown => todo!(),
            Message::Backspace => todo!(),
            Message::JumpWordRight => todo!(),
            Message::GotoStartOfLine => todo!(),
            Message::GotoEndOfLine => todo!(),
            Message::Enter => todo!(),
            _ => unreachable!(),
        }
    }
}
