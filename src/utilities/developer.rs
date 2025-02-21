//! A utility for testing

use std::io::stdout;

use nix::unistd::Pid;
use ratatui::{style::{Color, Style}, widgets::{Clear, Paragraph, Wrap}};
use indoc::indoc;

use crate::{model::Message, notification::Notification};

pub struct DeveloperModel();

impl super::Utility for DeveloperModel {
    fn update(&mut self, msg: crate::model::Message) -> Option<Message> {
        match msg {
            Message::InsertChar(char) => {
                match char {
                    '\n' => Some(Message::Notification(indoc!{"
                        warning: unused variable: `width`
                        --> src/view.rs:139:21
                            |
                        139 |                 let width = wrapped_content.lines();
                            |                     ^^^^^ help: if this is intentional, prefix it with an underscore: `_width`
                            |
                            = note: `#[warn(unused_variables)]` on by default"}.to_owned()
                        , Style::new().bg(Color::Red)
                    )),
                    'z' => {
                        crate::suspend::suspend().unwrap();
                        Some(Message::CloseUtility)
                    }
                    _ => None
                }
            },
            msg => Some(msg)
        }
    }
    fn view(&self, m: &crate::model::Model, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        super::default_view("brrrrr", indoc! {"
        * n - create an error notification
        * z - experiemntal suspend option
        "}, f, area);
    }
}
