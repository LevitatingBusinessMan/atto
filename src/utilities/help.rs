use indoc::indoc;

use super::Utility;

pub struct HelpModel();

impl Utility for HelpModel {
    fn view(&self, m: &crate::model::Model, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        super::default_view("Help", indoc! {"
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
        C-b Shell
       "}, f, area);
    }
}
