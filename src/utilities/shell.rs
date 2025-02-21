use std::{env, io::{self, stdout, BufRead, BufReader, Read, Stdout, Write}, os::fd::{AsRawFd, BorrowedFd}, process::{self, Command, Stdio}};

use crossterm::{event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, KeyboardEnhancementFlags, PushKeyboardEnhancementFlags}, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}, ExecutableCommand, QueueableCommand};
use nix::{libc::POLLIN, poll::{poll, PollFd, PollFlags, PollTimeout}, sys::{select::FdSet, time::TimeVal}};
use ratatui::{layout::{Constraint, Direction, Layout, Rect}, style::{Color, Style}, widgets::{Clear, Paragraph}, Frame};
use tracing::{debug, error, warn};

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
        crate::tui::restore()?;
        terminal.clear()?;
        terminal.set_cursor_position((0,0))?;

        let mut child = cmd.arg(&self.entry)
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn().log()?;

        self.entry.clear();

        // we could potentially improve performance by using a bufreader
        let mut stdout_pipe = child.stdout.take().unwrap();
        let mut stderr_pipe = child.stderr.take().unwrap();

        let mut stdout_buf = [0; 1024];
        let mut stderr_buf = [0; 1024];

        let mut stdout = vec![];
        let mut stderr = vec![];

        let mut pollfds = vec![
            PollFd::new(unsafe { BorrowedFd::borrow_raw(stdout_pipe.as_raw_fd()) }, PollFlags::POLLIN),
            PollFd::new(unsafe { BorrowedFd::borrow_raw(stderr_pipe.as_raw_fd()) }, PollFlags::POLLIN),
        ];

        loop {
            match child.try_wait()? {
                Some(status) => {
                    terminal.clear()?;
                    crate::tui::setup()?;
                    debug!("Exited with status {:?}", status.code());
                    stdout_pipe.read_to_end(&mut stdout)?;
                    stderr_pipe.read_to_end(&mut stderr)?;
                    debug!("Total {}b stdout, {}b stderr", stdout.len(), stderr.len());
                    
                    let stdout = String::from_utf8(stdout);
                    let stderr = String::from_utf8(stderr);

                    /*
                    I really need to fix the output formatting here.
                    If the stdout and stderr are empty user should still get feedback.
                    The utf8 should never be invalid. I feel like that would be worthy of an error.
                    When there is both stdout and stderr they should've preferable been mixed
                    beforehand.
                     */

                    if stdout.is_err() || stderr.is_err() {
                        warn!("Failed to utf8 parse command output");
                        match status.success() {
                            true => {
                                return Ok(Message::Notification(
                                    format!("Command executed succesfully, but output was not utf8"),
                                    Style::new().bg(Color::Green).fg(Color::White)
                                ));
                            },
                            false => {
                                return Ok(Message::Notification(
                                    format!("Command failed with code {:?}, output was not utf8", status.code()),
                                    Style::new().bg(Color::Red).fg(Color::White)
                                ));
                            }
                        }
                    } else {
                        let style = if status.success(){
                            Style::new().bg(Color::White).fg(Color::Black)
                        }
                        else {
                            Style::new().bg(Color::Red).fg(Color::White)
                        };
                        let stdout = stdout.unwrap();
                        let stderr = stderr.unwrap();
                        let display = if stderr.is_empty() || stdout.is_empty() {
                            format!("{}{}", stdout.trim(), stderr.trim()) }
                        else {
                            format!("{}\n{}", stdout.trim(), stderr.trim())
                        };
                
                        return Ok(Message::Notification(display, style))
                    }
                },
                None => {
                    pollfds[0].set_events(PollFlags::POLLIN);
                    pollfds[1].set_events(PollFlags::POLLIN);
                    if poll(&mut pollfds, Some(10_u8))? > 0 {
                        if pollfds[0].any().unwrap() {
                            let n = stdout_pipe.read(&mut stdout_buf)?;
                            debug!("received {} bytes in stdout", n);
                            stdout.extend_from_slice(&stdout_buf[..n]);
                            io::stdout().write_all(&stdout_buf[..n])?;
                        }
                        if pollfds[1].any().unwrap() {
                            let n = stderr_pipe.read(&mut stderr_buf)?;
                            debug!("received {} bytes in stderr", n);
                            stderr.extend_from_slice(&stderr_buf[..n]);
                            io::stderr().write_all(&stderr_buf[..n])?;
                        }
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
