use indoc::indoc;
use ratatui::widgets::{Clear, Paragraph, Wrap};

use super::Utility;

pub struct HelpModel();

impl Utility for HelpModel {
    fn view(&self, m: &crate::model::Model, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        f.render_widget(Clear, area);
        f.render_widget(
        Paragraph::new(indoc! {"
            Welcome to Atto!
            Here is a list of keybinds:
            C-c Copy
            C-x Cut
            C-v Paste
            C-a Select All
            A-a Start
            A-e End
            A-j Right
            A-i Up
            A-f Left
            A-n Down
            C-f Find
            C-e Command
        "})
        .block(
            super::default_block("Help")
        )
        .wrap(Wrap { trim: false })
    , area);
    }
}
