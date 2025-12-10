pub mod help;
pub mod find;
pub mod confirm;
pub mod developer;
pub mod shell;

use ratatui::{Frame, layout::Rect, style::{Style, Stylize}, widgets::{Block, Borders, Paragraph}};

use crate::model::{Message, Model};

/// All utilities must implement this trait
pub trait Utility {
    /// Receive a message
    /// The utility may choose to discard, forward or replace it
    /// (this can probably take a reference to the model if necessary)
    fn update(&mut self, msg: Message) -> Option<Message> {
        Some(msg)
    }
    /// Given the maximum allowed area size
    /// the widget can draw itself
    fn view(&self, f: &mut Frame, area: Rect);
}

/// Utilitis are encouraged to render themselves in this block
pub fn default_block<'a>(name: &'a str) -> Block<'a> {
    Block::default()
    .title(name)
    .borders(Borders::ALL)
    //.padding(Padding::uniform(1))
    .border_style(Style::new().blue())
}

pub fn default_view(title: &str, content: &str, f: &mut Frame, area: Rect) {
    use ratatui::layout::{Layout, Constraint, Direction};
    use ratatui::widgets::{Clear, Paragraph};
    let block = default_block(title);
    let widget_content = textwrap::fill(content, block.inner(area).width as usize);
    let height = widget_content.lines().count();
    let bordersandpadding = area.height - block.inner(area).height;
    let area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(height as u16 + bordersandpadding), Constraint::Min(0)])
        .split(area)[0];
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(widget_content).block(block), area);
}

/// The top right window
pub enum UtilityWindow {
    Help(help::HelpModel),
    Find(find::FindModel),
    Confirm(confirm::ConfirmModel),
    Developer(developer::DeveloperModel),
    Shell(shell::ShellModel),
}
