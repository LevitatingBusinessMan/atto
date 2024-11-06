use std::{env, process::{self, Command, Stdio}};

use ratatui::{layout::Rect, style::{Color, Style}, widgets::{Clear, Paragraph}, Frame};
use tracing::{debug, error};

use crate::model::{Message, Model};

#[derive(Debug)]
pub struct ShellModel {
    pub entry: String,
}

impl ShellModel {
    pub fn new() -> Self {
        Self { entry: String::new() }
    }

    #[tracing::instrument(skip_all, level="debug", fields(cmd=self.entry))]
    fn exec(&mut self) -> Message {
        let mut shell: Command;
        let cmd = if cfg!(target_os = "windows") {
            shell = Command::new("cmd");
            shell.arg("/C")
        } else {
            shell = Command::new("sh");
            shell.arg("-c")
        };

        let res = cmd.arg(&self.entry).stdin(Stdio::inherit()).output();

        self.entry.clear();

        match res {
            Ok(output) => {
                debug!("Exited with status {:?}", output.status.code());
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                Message::Notification(
                    if stderr.is_empty() || stdout.is_empty() { format!("{}{}", stdout.trim(), stderr.trim()) }
                    else { format!("{}\n{}", stdout.trim(), stderr.trim())},
                    if output.status.success() { Style::new().bg(Color::White).fg(Color::Black) }
                    else { Style::new().bg(Color::Red) }
                )
            },
            Err(err) => {
                error!("{err:?}");
                Message::Notification(format!("{err:?}"), Style::new().bg(Color::Red))
            },
        }

    }
}

impl super::Utility for ShellModel {
    fn update(&mut self, msg: Message) -> Option<Message> {
        match &msg {
            Message::InsertChar(c) => self.entry.push(*c),
            Message::Paste(paste) => self.entry.push_str(paste),
            Message::Backspace => { self.entry.pop(); },
            Message::Enter => return Some(self.exec()),
            _ => return Some(msg),
        }
        None
    }

    fn view(&self, m: &Model, f: &mut Frame, area: Rect) {
        f.render_widget(Clear, area);
        let block = super::default_block("Shell");
        f.render_widget(Paragraph::new(self.entry.clone()).block(block), area);
    }
}
