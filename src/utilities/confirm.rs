use ratatui::widgets::{Clear, Paragraph};
use tracing::{debug, error};

use crate::model::Message;
use indoc::indoc;

pub struct ConfirmModel {
    pub msg: String,
    pub choices: Vec<(char, Message)>
}

impl ConfirmModel {
    pub fn new(msg: String, choices: Vec<(char, Message)>) -> Self {
        Self {msg, choices}
    }
}

impl super::Utility for ConfirmModel {
    fn update(&mut self, msg: Message) -> Option<Message> {
        match msg {
            Message::InsertChar(c) => {
                for (choice, action) in self.choices.iter() {
                    if *choice == c {
                        return Some(Message::CloseUtilityAnd(Box::new(action.clone())));
                    }
                }
                return None
            },
            msg => Some(msg)
        }
    }
    
    fn view(&self, m: &crate::model::Model, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        f.render_widget(Clear, area);
        let block = super::default_block("Confirm");
        let width = block.inner(area).width as usize;
        let choicesstring= self.choices.iter().map(|(c, _a)| c.to_string()).collect::<Vec<_>>().join("/");
        f.render_widget(Paragraph::new(textwrap::fill(&mut format!(indoc! {"
        {}: {}
        "}, self.msg, choicesstring), width)).block(block), area);
    }
}
