
use crate::model::Message;

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
                        return Some(Message::Double(
                            Box::new(Message::CloseUtility),
                            Box::new(action.clone())
                        ));
                    }
                }
                return None
            },
            msg => Some(msg)
        }
    }

    fn view(&self, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let choicesstring= self.choices.iter().map(|(c, _a)| c.to_string()).collect::<Vec<_>>().join("/");
        let content =  format!("{}: {}", self.msg, choicesstring);
        super::default_view("Confirm", &content, f, area);
    }
}
