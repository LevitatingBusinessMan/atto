use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style, Stylize}, text::{Line, Span}, widgets::{Clear, Paragraph, Wrap}};
use tracing::trace;

use crate::{model::{Message, Model}, utilities};

pub struct FindModel {
    pub entry: String,
    pub occurences: Option<usize>,
}

impl FindModel {
    pub fn new() -> Self {
        Self {
            entry: String::new(),
            occurences: None
        }
    }
}

impl utilities::Utility for FindModel {
    fn view(&self, f: &mut Frame, area: Rect) {
        let title = format!("Find ({})", self.occurences.unwrap_or(0));
        super::default_view(&title, &self.entry, f, area);
   }

   fn update(&mut self, msg: Message) -> Option<Message> {
       match msg {
           Message::OpenFind => {
               // jump to next highlight
               None
           },
           Message::InsertChar(c) => {
               if c != '\n' {
                   self.entry.push(c);
                   Some(Message::Find(self.entry.clone()))
               } else {
                   None
               }
           },
           Message::Backspace => {
            self.entry.pop();
            Some(Message::Find(self.entry.clone()))
           },
           // we could do a thing where if it receives an ambigious Message:Next
           // it can choose to replace it with a Message:NextSelection or Message::NextHighlight
           msg => Some(msg),
       }
   }
}
