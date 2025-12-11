use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style, Stylize}, text::{Line, Span}, widgets::{Clear, Paragraph, Wrap}};
use tracing::trace;

use crate::{model::{Message, Model}, utilities};

pub struct FindModel {
    pub entry: super::EntryModel,
    pub occurences: Option<usize>,
}

impl FindModel {
    pub fn new() -> Self {
        Self {
            entry: super::EntryModel::new(),
            occurences: None
        }
    }
}

impl utilities::Utility for FindModel {
    fn view(&self, f: &mut Frame, area: Rect) {
        let title = format!("Find ({})", self.occurences.unwrap_or(0));
        super::default_view(&title, &self.entry.text, f, area);
   }

   fn update(&mut self, msg: Message) -> Option<Message> {
       let old = self.entry.text.clone();
       let msg = self.entry.update(msg);

       if self.entry.text != old && !self.entry.text.is_empty() {
           return Some(Message::Find(self.entry.text.clone()))
       }

       if msg.is_none() {
           if self.entry.text != old && !self.entry.text.is_empty() {
               return Some(Message::Find(self.entry.text.clone()))
           } else {
               return None
           }
       }

       return match msg.unwrap() {
           Message::OpenFind | Message::Enter => {
               Some(Message::JumpNextHighlight)
           },
           msg => Some(msg),
       }
   }
}
