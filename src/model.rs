use std::rc::Rc;

use ratatui::{Frame, layout::{Layout, Constraint}, widgets::{Block, Paragraph, Borders}, style::{Style, Stylize}, text::Line};

use crate::buffer::Buffer;

pub struct Model {
    /// What buffer is selected
    selected: usize,
    /// What buffers are open
    buffers: Rc<Vec<Buffer>>,
    /// If we should close the application
    pub running: bool,
}

impl Model {

    pub fn new(buffers: Vec<Buffer>) -> Self {
        Model {
            buffers: Rc::new(buffers),
            selected: 0,
            running: true
        }
    }

    pub fn update(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::NextBuffer => self.selected = (self.selected + 1) % self.buffers.len(),
            Message::PreviousBuffer => self.selected = (self.selected + self.buffers.len() - 1) % self.buffers.len(),
            Message::Quit => self.running = false,
        }
        None
    }

    pub fn view(&self, f: &mut Frame) {
        let main = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([Constraint::Max(1000), Constraint::Length(1)])
                .split(f.size());

            f.render_widget(
                Paragraph::new(String::from_utf8_lossy(&self.buffers[self.selected].content))
                    .block(Block::default()
                    .title(self.buffers[self.selected].name.clone())
                    .borders(
                            Borders::TOP | Borders::RIGHT | Borders::LEFT
                    )),
                    main[0]
            );
        
            f.render_widget(
                Paragraph::new(
                    Line::styled(
                        std::format!(
                            " {:<} {:>width$} ",
                            "Welcome to Atto!",
                            std::format!("[{}]", self.buffers.iter().map(|b| b.name.clone()).collect::<Vec<String>>().join("|")),
                            width = main[1].width as usize - "Welcome to Atto!".len() - 3
                        ),
                        Style::default()
                        .black()
                        .on_white()
                    )
                ),
                main[1]
            )
    }
}
pub enum Message {
    NextBuffer,
    PreviousBuffer,
    Quit,
}
