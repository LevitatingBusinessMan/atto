use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style, Stylize}, text::{Line, Span}, widgets::{Clear, Paragraph, Wrap}};
use tracing::trace;

use crate::{model::{Message, Model}, utilities::{self, EntryModel}};

pub struct SaveAsModel {
    pub entry: EntryModel,
}

impl SaveAsModel {
    pub fn new() -> Self {
        Self {
            entry: super::EntryModel::new(),
        }
    }
}

impl utilities::Utility for SaveAsModel {
    fn view(&self, f: &mut Frame, area: Rect) {
        super::default_view(&"Save as", &self.entry.text, f, area);
   }

   fn update(&mut self, msg: Message) -> Option<Message> {
       let msg = self.entry.update(msg);

       return match msg {
           Some(msg) => match msg {
                Message::Enter => {
                    if !self.entry.text.is_empty() {
                        Some(Message::Double(
                            Box::new(Message::CloseUtility),
                            Box::new(Message::SaveAs(self.entry.text.clone()))
                        ))
                    } else {
                        None
                    }
                },
                msg => Some(msg),
           },
           None => None,
       }
   }
}
