use std::{env, io::{self, stdout, Read, Stdout, Write}, process::{self, Command, Stdio}};

use crossterm::{event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, KeyboardEnhancementFlags, PushKeyboardEnhancementFlags}, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}, ExecutableCommand, QueueableCommand};
use ratatui::{layout::{Constraint, Direction, Layout, Rect}, style::{Color, Style}, widgets::{Clear, Paragraph}, Frame};
use tracing::{debug, error};

use crate::{logging::LogError, model::{Message, Model}, TERMINAL};

use super::default_view;

//static UNIX_SHELL: &'static str = "sh";
static UNIX_SHELL: &'static str = "fish";

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
            shell = Command::new(UNIX_SHELL);
            shell.arg("-c")
        };

        let mut terminal = crate::TERMINAL.get().unwrap().lock().unwrap();

        tracing::warn!("Clearing terminal");
        terminal.clear()?;
        terminal.set_cursor_position((0,0))?;
        crate::tui::restore()?;
        
        let mut child = cmd.arg(&self.entry)
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn().log()?;

        self.entry.clear();

        let mut stdout_pipe = child.stdout.take().unwrap();
        let mut stderr_pipe = child.stderr.take().unwrap();

        let mut stdout_buf = [0; 1024];
        let mut stderr = Vec::with_capacity(256);

        let mut stdout = vec![];

        let mut firstread = true;

        loop {
            let stdout_read = stdout_pipe.read(&mut stdout_buf)?;
            let stderr_read = stderr_pipe.read(&mut stderr)?;
            match child.try_wait()? {
                Some(status) => {
                    {
                        io::stdout().execute(EnterAlternateScreen);
                        enable_raw_mode();
                        io::stdout().queue(EnableMouseCapture);
                        // https://docs.rs/crossterm/latest/crossterm/event/struct.KeyboardEnhancementFlags.html
                        io::stdout().queue(PushKeyboardEnhancementFlags(
                            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                            | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                            | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                        ));
                        io::stdout().queue(EnableBracketedPaste);
                    }

                    debug!("Exited with status {:?}, {}b stdout, {}b stderr", status.code(), stdout.len(), stderr.len());
                    stdout_pipe.read_to_end(&mut stdout)?;
                    stderr_pipe.read_to_end(&mut stderr)?;
                    let stdout = String::from_utf8_lossy(&stdout);
                    let stderr = String::from_utf8_lossy(&stderr);

                    let display = if stderr.is_empty() || stdout.is_empty() {
                        format!("{}{}", stdout.trim(), stderr.trim()) }
                    else {
                        format!("{}\n{}", stdout.trim(), stderr.trim())
                    };
            
                    let style = if status.success(){
                        Style::new().bg(Color::White).fg(Color::Black)
                    }
                    else {
                        Style::new().bg(Color::Red)
                    };
            
                    return Ok(Message::Notification(display, style))
                },
                None => {
                    if stdout_read > 0 || stderr_read > 0 {
                        tracing::debug!("Read {stdout_read}b from stdout and {stderr_read}b form stderr");
                        stdout.extend_from_slice(&stdout_buf[..stdout_read]);
                        if firstread {
                            firstread = false;
                        }
                        write!(io::stdout(), "{}", String::from_utf8_lossy(&stdout_buf[..stdout_read]))?;
                        //write!(io::stderr(), "{}", String::from_utf8_lossy(&stderr[stderr.len()-stderr_read..]))?;
                        stdout.reserve(stdout_read * 2);
                        stderr.reserve(stderr_read * 2);
                    }
                },
            }
        }
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
