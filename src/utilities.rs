pub mod help;
pub mod find;
pub mod confirm;
pub mod developer;

use ratatui::{layout::Rect, style::{Style, Stylize}, widgets::{Block, Borders, Padding}, Frame};

use crate::model::{Message, Model};

/// All utilities must implement this trait
pub trait Utility {
    /// Receive a message
    /// The utility may choose to discard, forward or replace it
    fn update(&mut self, msg: Message) -> Option<Message> {
        Some(msg)
    }
    /// Given the maximum allowed area size
    /// the widget can draw itself
    fn view(&self, m: &Model, f: &mut Frame, area: Rect);
}

/// Utilitis are encouraged to render themselves in this block
pub fn default_block<'a>(name: &'a str) -> Block<'a> {
    Block::default()
    .title(name)
    .borders(Borders::ALL)
    .padding(Padding::uniform(1))
    .border_style(Style::new().blue())
}

/// The top right window
pub enum UtilityWindow {
    Help(help::HelpModel),
    Find(find::FindModel),
    Confirm(confirm::ConfirmModel),
    Developer(developer::DeveloperModel),
}
