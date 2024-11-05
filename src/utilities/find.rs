use ratatui::{layout::{Constraint, Layout, Rect}, style::{Modifier, Style, Stylize}, text::{Line, Span}, widgets::{Block, Clear, Padding, Paragraph, Wrap}, Frame};

use crate::{model::{Message, Model}, utilities};

pub struct FindModel {
    pub entry: String,
}

impl FindModel {
    pub fn new() -> Self {
        Self { entry: String::new() }
    }
}

impl utilities::Utility for FindModel {
    fn view(&self, m: &Model, f: &mut Frame, area: Rect) {
        // it might need the main model to render
       // to get the list of occurences from
       // the active buffer

       f.render_widget(Clear, area);

       let block = utilities::default_block("Find");

       let layout = Layout::new(ratatui::layout::Direction::Vertical, [
           Constraint::Length(3),
           Constraint::Length(1), // Padding
           Constraint::Min(0),
       ]).split(block.inner(area));

       f.render_widget(block, area);

       let underlined = Style::default().add_modifier(Modifier::UNDERLINED);
       let search_entry = match self.entry.len() {
           0 => Span::styled(" ", underlined.fg(ratatui::style::Color::Gray)),
           _ => Span::styled(self.entry.clone(), underlined),
       };

       f.render_widget(
           Paragraph::new(search_entry)
           .wrap(Wrap { trim: true }),
           layout[0]
       );

       let occurences_str = format!("Found {}", m.current_buffer().highlights.len());
       let occurences = Line::raw(occurences_str);
       f.render_widget(occurences, layout[2]);
   }

   fn update(&mut self, msg: Message) -> Option<Message> {
       match msg {
           Message::InsertChar(c) => {
               self.entry.push(c);
               Some(Message::Find(self.entry.clone()))
           },
           // we could do a thing where if it receives an ambigious Message:Next
           // it can choose to replace it with a Message:NextSelection
           msg => Some(msg),
       }
   }
}
