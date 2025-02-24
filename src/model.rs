use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Mutex};

use ratatui::{layout::Size, prelude::Backend, style::{Color, Style}, Terminal};
use syntect::{highlighting::{ThemeSet, Theme}, parsing::SyntaxSet};
use tracing::{debug, error};

use crate::{buffer::{self, Buffer}, logging::LogError, utilities::{self, developer::DeveloperModel, Utility, UtilityWindow}};
use crate::parse::ParseCache;
use crate::notification::Notification;

pub struct Model {
    /// What buffer is selected
    pub selected: usize,
    /// What buffers are open
    pub buffers: Vec<Buffer>,
    /// If we should close the application
    pub running: bool,
    /// The utility window
    pub utility: Option<UtilityWindow>,
    /// Tell the view it may have to scroll
    /// the buffer because the cursor might've moved 
    /// out of view; 
    pub may_scroll: bool,
    pub theme_set: ThemeSet,
    pub syntax_set: SyntaxSet,
    pub theme: String,
    pub viewport: Size,
    pub notification: Option<Notification>,
    /// visualize whitespace
    pub show_whitespace: bool,
}

impl Model {
    pub fn new<'a>(mut buffers: Vec<Buffer>, theme_set: ThemeSet, viewport: Size) -> Model {
        // let parse_caches = (|| {
        //     let mut map = HashMap::new();
        //     for buf in &buffers {
        //         map.insert(buf.name.clone().unwrap_or("?".to_string()), Rc::new(RefCell::new(ParseCache::new())));
        //     }
        //     map
        // })();

        let syntax_set = SyntaxSet::load_defaults_newlines();
        for buffer in &mut buffers {
            buffer.find_syntax(&syntax_set);
        }
        Model {
            buffers: buffers,
            selected: 0,
            running: true,
            utility: None,
            may_scroll: false,
            theme_set,
            syntax_set,
            theme: "dracula".to_owned(),
            viewport,
            notification: None,
            show_whitespace: false,
        }
    }

    #[tracing::instrument(skip(self), level="debug")]
    pub fn update(&mut self, msg: Message) -> Option<Message> {
        // remove notification if elapsed
        // (the handling of this makes it so that if the user does not somehow create a message)
        // the notification will hang around
        if let Some(notification) = &self.notification {
            if notification.expired() {
                self.notification = None;
            }
        }

        let new_msg = match &mut self.utility {
            Some(UtilityWindow::Find(find)) => find.update(msg),
            Some(UtilityWindow::Help(help)) => help.update(msg),
            Some(UtilityWindow::Confirm(confirm)) => confirm.update(msg),
            Some(UtilityWindow::Developer(developer)) => developer.update(msg),
            Some(UtilityWindow::Shell(shell)) => shell.update(msg),
            None => Some(msg),
        };

        if new_msg.is_none() {
            return None
        }

        let msg = new_msg.unwrap();

        match msg {
            Message::NoMessage => {},
            Message::NextBuffer => self.selected = (self.selected + 1) % self.buffers.len(),
            Message::PreviousBuffer => self.selected = (self.selected + self.buffers.len() - 1) % self.buffers.len(),
            Message::QuitNoSave => self.running = false,
            Message::Quit => {
                match self.current_buffer().dirty() {
                    Ok(true) => {
                        self.utility = Some(UtilityWindow::Confirm(
                            utilities::confirm::ConfirmModel::new(
                                String::from("There are unsaved changes. Do you want to save?"),
                                vec![
                                    ('y', Message::Double(Box::new(Message::Save), Box::new(Message::Quit))),
                                    ('n', Message::QuitNoSave),
                                ]
                        )));
                    },
                    Ok(false) => self.running = false,
                    Err(err) => {
                        error!("{err:?}");
                        self.running = false;
                    },
                }
            },
            Message::ScrollDown => {
                if (self.current_buffer().content.lines().count() + 1 - self.viewport.height as usize) > self.current_buffer_mut().top {
                    self.current_buffer_mut().top += 2;
                }
            },
            Message::ScrollUp => self.current_buffer_mut().top = self.current_buffer_mut().top.saturating_sub(2),
            Message::OpenHelp => self.utility = Some(UtilityWindow::Help(utilities::help::HelpModel())),
            Message::OpenFind => self.utility = Some(UtilityWindow::Find(utilities::find::FindModel::new())),
            Message::Escape => return Some(Message::CloseUtility),
            Message::CloseUtility => self.utility = None,
            Message::InsertChar(chr) => {
                self.current_buffer_mut().insert(chr);
                self.may_scroll = true;
            },
            Message::MoveLeft => {
                self.current_buffer_mut().move_left();
                self.may_scroll = true;
            },
            Message::MoveRight => {
                self.current_buffer_mut().move_right();
                self.may_scroll = true;
            },
            Message::MoveUp => {
                self.current_buffer_mut().move_up();
                self.may_scroll = true;
            },
            Message::MoveDown => {
                self.current_buffer_mut().move_down();
                self.may_scroll = true;
            },
            Message::PageUp => {
                let height = self.viewport.height as usize;
                self.current_buffer_mut().page_up(height);
                // self.may_scroll = true;
            },
            Message::PageDown => {
                let height = self.viewport.height as usize;
                self.current_buffer_mut().page_down(height);
                // self.may_scroll = true;
            },
            Message::Backspace => self.current_buffer_mut().backspace(),
            Message::Delete => self.current_buffer_mut().delete(),
            Message::JumpWordLeft => {
                self.current_buffer_mut().move_word_left();
                self.may_scroll = true;
            },
            Message::JumpWordRight => {
                self.current_buffer_mut().move_word_right();
                self.may_scroll = true;
            },
            Message::GotoStartOfLine => self.current_buffer_mut().goto_start_of_line(),
            Message::GotoEndOfLine => self.current_buffer_mut().goto_end_of_line(),
            Message::Enter => return Some(Message::InsertChar('\n')),
            Message::Find(query) => {
                self.current_buffer_mut().find(query);
                self.may_scroll = true;
            },
            Message::Save => {
                if let Err(e) =  self.current_buffer_mut().save() {
                    tracing::warn!("{:?}", e);
                    return Some(Message::Notification(
                        format!("Error writing file: {e}"),
                        Style::new().bg(Color::Red).fg(Color::White)
                    ));
                } else {
                    return Some(Message::Notification(
                        String::from("SAVED"),
                        Style::new().bg(Color::Green).fg(Color::Black)
                    ));
                }
            },
            Message::SaveAsRoot => {
                if let Err(e) = self.current_buffer_mut().save_as_root() {
                    tracing::error!("Error saving as root: {e:?}");
                    return Some(Message::Notification(
                        format!("Error saving as root: {e}"),
                        Style::new().bg(Color::Red).fg(Color::White)
                    ));
                } else {
                    return Some(Message::Notification(
                        String::from("SAVED AS ROOT"),
                        Style::new().bg(Color::Yellow).fg(Color::Black)
                    ));
                }
            },
            Message::Resize(x, y) => {
                self.viewport = (x,y).into();
            },
            Message::MouseLeft(x, y) => {
                self.current_buffer_mut().set_viewport_cursor_pos(x, y);
            },
            Message::Notification(content, style) => {
                self.notification = Some(Notification::new(content, style));
            },
            Message::DeveloperKey => {
                self.utility = Some(UtilityWindow::Developer(DeveloperModel()));
            },
            Message::Paste(paste) => self.current_buffer_mut().paste(&paste),
            Message::OpenShell => self.utility = Some(utilities::UtilityWindow::Shell(utilities::shell::ShellModel::new())),
            Message::Double(first, second) => {
                self.update(*first);
                return Some(*second);
            },
            Message::SaveAsRootConfirmation => {
                self.utility = Some(UtilityWindow::Confirm(
                    utilities::confirm::ConfirmModel::new(
                        format!("Do you want to save this file using {}?", buffer::PRIVESC_CMD),
                        vec![
                            ('y', Message::SaveAsRoot),
                            ('n', Message::NoMessage)
                        ]
                )));
            },
            Message::ToBottom => {
                self.current_buffer_mut().to_bottom();
                self.may_scroll = true;
            },
            Message::ToTop => {
                self.current_buffer_mut().to_top();
                self.may_scroll = true;
            },
            Message::Tab => {
                self.current_buffer_mut().insert('\t');
                self.may_scroll = true;
            },
            Message::Suspend => match crate::suspend::suspend().log() {
                Ok(_) => {},
                Err(e) => {
                    let _ = crate::tui::setup();
                    let _ = crate::TERMINAL.get().unwrap().lock().unwrap().clear();
                    return Some(Message::Notification(
                        format!("Suspendin failed with {:?}", e),
                        Style::new().bg(Color::Red).fg(Color::White)
                    ))
                },
            },
            Message::NewEmptyBuffer => {
                self.buffers.push(Buffer::empty());
                self.selected = self.buffers.len() - 1;
            }
        }
        None
    }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        return &mut self.buffers[self.selected];
    }

    pub fn current_buffer(&self) -> &Buffer {
        return &self.buffers[self.selected];
    }

    pub fn theme(&self) -> &Theme {
        return &self.theme_set.themes[&self.theme]
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    NextBuffer,
    PreviousBuffer,
    /// Attempt to quit (but may be stopped)
    Quit,
    ScrollDown,
    ScrollUp,
    OpenHelp,
    OpenFind,
    Find(String),
    Escape,
    InsertChar(char),
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Backspace,
    Delete,
    JumpWordLeft,
    JumpWordRight,
    GotoStartOfLine,
    GotoEndOfLine,
    Enter,
    Save,
    Resize(u16, u16),
    MouseLeft(u16, u16),
    Notification(String, Style),
    DeveloperKey,
    CloseUtility,
    /// Quit immediately
    QuitNoSave,
    Paste(String),
    OpenShell,
    /// Two messages
    Double(Box<Message>, Box<Message>),
    SaveAsRootConfirmation,
    SaveAsRoot,
    /// can be used just to force update() and view() to run
    NoMessage,
    ToTop,
    ToBottom,
    Tab,
    Suspend,
    NewEmptyBuffer,
}
