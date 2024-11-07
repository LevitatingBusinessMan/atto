use std::{env, io, process::{self, Command, Stdio}};

use crossterm::{event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags}, terminal::{disable_raw_mode, enable_raw_mode}};
use ratatui::{layout::{Constraint, Direction, Layout, Rect}, style::{Color, Style}, widgets::{Clear, Paragraph}, Frame};
use tracing::{debug, error};

use crate::{logging::{LogError}, model::{Message, Model}};

use super::default_view;

#[derive(Debug)]
pub struct ShellModel {
    pub entry: String,
}

impl ShellModel {
    pub fn new() -> Self {
        Self { entry: String::new() }
    }

    #[tracing::instrument(skip_all, level="info", fields(cmd=self.entry))]
    fn exec(&mut self) -> io::Result<Message> {
        let mut shell: Command;
        let cmd = if cfg!(target_os = "windows") {
            shell = Command::new("cmd");
            shell.arg("/C")
        } else {
            shell = Command::new("sh");
            shell.arg("-c")
        };

        let output = cmd.arg(&self.entry).stdin(Stdio::null()).output().log()?;
        debug!("Exited with status {:?}", output.status.code());

        self.entry.clear();

        debug!("{:?}", &output.stdout);

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let display_ = if stderr.is_empty() || stdout.is_empty() {
            format!("{}{}", stdout.trim(), stderr.trim()) }
        else {
            format!("{}\n{}", stdout.trim(), stderr.trim())
        };

        let style = if output.status.success(){
            Style::new().bg(Color::White).fg(Color::Black)
        }
        else {
            Style::new().bg(Color::Red)
        };

        debug!("{:?}", Message::Notification(display_.clone(), style));
        Ok(Message::Notification(display_, style))
    }
}

impl super::Utility for ShellModel {
    fn update(&mut self, msg: Message) -> Option<Message> {
        match &msg {
            Message::InsertChar(c) => self.entry.push(*c),
            Message::Paste(paste) => self.entry.push_str(paste),
            Message::Backspace => { self.entry.pop(); },
            Message::Enter => return match self.exec().log() {
                Ok(m) => Some(m),
                Err(e) => Some(Message::Notification(format!("{e:?}"), Style::new().bg(Color::Red)))
            },
            _ => return Some(msg),
        }
        None
    }

    fn view(&self, m: &Model, f: &mut Frame, area: Rect) {
        super::default_view("Shell", &self.entry, f, area);
    }
}
