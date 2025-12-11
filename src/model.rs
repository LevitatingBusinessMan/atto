use std::{io::stdout, rc::Rc};

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture}, ExecutableCommand};
use ratatui::{layout::Size, prelude::Backend, style::{Color, Style}};
use syntect::{highlighting::{ThemeSet, Theme}, parsing::SyntaxSet};
use tracing::{debug, error, trace};

use crate::{buffer::{self, Buffer}, logging::LogError, undo::UndoState, utilities::{self, Utility, UtilityWindow, developer::DeveloperModel, save_as::SaveAsModel}};
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
    pub theme_set: ThemeSet,
    pub syntax_set: SyntaxSet,
    pub theme: String,
    pub viewport: Size,
    pub notification: Option<Notification>,
    /// visualize whitespace
    pub show_whitespace: bool,
    /// is mouse_capture enabled
    pub mouse_capture: bool,
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
            theme_set,
            syntax_set,
            theme: "dracula".to_owned(),
            viewport,
            notification: None,
            show_whitespace: false,
            mouse_capture: true,
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
            Some(UtilityWindow::SaveAs(save_as)) => save_as.update(msg),
            None => Some(msg),
        };

        if new_msg.is_none() {
            return None;
        }

        let msg = new_msg.unwrap();

        if self.utility.is_some() {
            debug!("Utility {:?} returned {:?}", &self.utility.as_ref().unwrap(), &msg);
        }

        // If the view should be altered
        let mut scroll_view = false;
        let mut center_view = false;
        let mut next_msg = None;

        // Finally evaluate the message
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
                // TODO fix substract with overflow error
                if (self.current_buffer().content.lines().count() + 1 - self.viewport.height as usize) > self.current_buffer_mut().top {
                    self.current_buffer_mut().top += 2;
                }
            },
            Message::ScrollUp => self.current_buffer_mut().top = self.current_buffer_mut().top.saturating_sub(2),
            Message::OpenHelp => self.utility = Some(UtilityWindow::Help(utilities::help::HelpModel())),
            Message::OpenFind => self.utility = Some(UtilityWindow::Find(utilities::find::FindModel::new())),
            Message::Escape => next_msg = Some(Message::CloseUtility),
            Message::CloseUtility => self.utility = None,
            Message::InsertChar(chr) => {
                self.current_buffer_mut().insert(chr);
                self.current_buffer_mut().undo.r#do(Message::InsertChar(chr), None);
                scroll_view = true;
            },
            Message::MoveLeft => {
                self.current_buffer_mut().move_left();
                scroll_view = true;
            },
            Message::MoveRight => {
                self.current_buffer_mut().move_right();
                scroll_view = true;
            },
            Message::MoveUp => {
                self.current_buffer_mut().move_up();
                scroll_view = true;
            },
            Message::MoveDown => {
                self.current_buffer_mut().move_down();
                scroll_view = true;
            },
            Message::PageUp => {
                let height = self.viewport.height as usize;
                self.current_buffer_mut().page_up(height);
                // scroll_view = true;
            },
            Message::PageDown => {
                let height = self.viewport.height as usize;
                self.current_buffer_mut().page_down(height);
                // scroll_view = true;
            },
            Message::Backspace => {
                let removed = self.current_buffer_mut().backspace();
                self.current_buffer_mut().undo.r#do(Message::Backspace, Some(removed));
            },
            Message::Delete => {
                let removed = self.current_buffer_mut().delete();
                self.current_buffer_mut().undo.r#do(Message::Delete, Some(removed));
            },
            Message::JumpWordLeft => {
                self.current_buffer_mut().move_word_left();
                scroll_view = true;
            },
            Message::JumpWordRight => {
                self.current_buffer_mut().move_word_right();
                scroll_view = true;
            },
            Message::GotoStartOfLine => self.current_buffer_mut().goto_start_of_line(),
            Message::GotoEndOfLine => self.current_buffer_mut().goto_end_of_line(),
            Message::Enter => next_msg = Some(Message::InsertChar('\n')),
            Message::Find(query) => {
                let occurences = self.current_buffer_mut().highlight(query);
                // if the find utility is open, set the occurences
                if let Some(UtilityWindow::Find(find)) = &mut self.utility {
                    find.occurences = Some(occurences);
                }
               next_msg = Some(Message::JumpNextHighlight);
            },
            Message::Save => {
                if self.current_buffer().name.is_none() {
                    self.utility = Some(UtilityWindow::SaveAs(SaveAsModel::new()));
                    next_msg = None
                } else {
                    if let Err(e) = self.current_buffer_mut().save() {
                        tracing::warn!("{:?}", e);
                        next_msg = Some(Message::Notification(
                            format!("Error writing file: {e}"),
                            Style::new().bg(Color::Red).fg(Color::White)
                        ));
                    } else {
                        next_msg = Some(Message::Notification(
                            String::from("SAVED"),
                            Style::new().bg(Color::Green).fg(Color::Black)
                        ));
                    }
                }
            },
            Message::SaveAsRoot => {
                if let Err(e) = self.current_buffer_mut().save_as_root() {
                    tracing::error!("Error saving as root: {e:?}");
                    next_msg = Some(Message::Notification(
                        format!("Error saving as root: {e}"),
                        Style::new().bg(Color::Red).fg(Color::White)
                    ));
                } else {
                    next_msg = Some(Message::Notification(
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
                if let Some(msg) = self.update(*first) {
                    next_msg = Some(Message::Double(Box::new(msg), Box::new(*second)));
                } else {
                    next_msg = Some(*second);
                }
            },
            Message::SaveAsRootConfirmation => {
                self.utility = Some(UtilityWindow::Confirm(
                    utilities::confirm::ConfirmModel::new(
                        format!("Do you want to save this file using {}?", *buffer::PRIVESC_CMD),
                        vec![
                            ('y', Message::SaveAsRoot),
                            ('n', Message::NoMessage)
                        ]
                )));
            },
            Message::ToBottom => {
                self.current_buffer_mut().to_bottom();
                scroll_view = true;
            },
            Message::ToTop => {
                self.current_buffer_mut().to_top();
                scroll_view = true;
            },
            Message::Tab => {
                self.current_buffer_mut().insert('\t');
                scroll_view = true;
            },
            Message::Suspend => match crate::suspend::suspend().log() {
                Ok(_) => {},
                Err(e) => {
                    let _ = crate::tui::setup();
                    let _ = crate::TERMINAL.get().unwrap().lock().unwrap().clear();
                    next_msg = Some(Message::Notification(
                        format!("Suspendin failed with {:?}", e),
                        Style::new().bg(Color::Red).fg(Color::White)
                    ))
                },
            },
            Message::NewEmptyBuffer => {
                self.buffers.push(Buffer::empty());
                self.selected = self.buffers.len() - 1;
            },
            Message::ToggleMouseCapture => {
                if self.mouse_capture {
                    stdout().execute(DisableMouseCapture);
                    self.mouse_capture = false;
                } else {
                    stdout().execute(EnableMouseCapture);
                    self.mouse_capture = true;
                }
            },
            Message::DragMouseLeft => {},
            Message::JumpNextHighlight => {
                self.current_buffer_mut().jump_next_highlight();
                center_view = true;
            },
            Message::JumpPreviousHighlight => {
                self.current_buffer_mut().jump_previous_highlight();
                center_view = true;
            },
            Message::SaveAs(path) => {
                let old = self.current_buffer().name.clone();
                self.current_buffer_mut().name = Some(path);
                match self.current_buffer_mut().save() {
                    Ok(()) => {
                        next_msg = Some(Message::Notification(
                            String::from("SAVED"),
                            Style::new().bg(Color::Green).fg(Color::Black)
                        ));
                    },
                    Err(e) => {
                        // revert old name/path
                        self.current_buffer_mut().name = old;
                        tracing::warn!("{:?}", e);
                        next_msg = Some(Message::Notification(
                            format!("Error writing file: {e}"),
                            Style::new().bg(Color::Red).fg(Color::White)
                        ));
                    }
                }
            },
            Message::UndoPaste(n) => {
                todo!()
            },
            Message::UndoBackspace(grapheme)  => {
                self.current_buffer_mut().paste(&grapheme);
            },
            Message::UndoDelete(grapheme) => {
                self.current_buffer_mut().insert_str(&grapheme);
            },
            Message::UndoInsertChar => {
                self.current_buffer_mut().delete();
            },
            Message::Redo => {
                todo!()
            },
            Message::Undo => {
                todo!()
            },
        };

        if center_view {
            self.center_view();
        }

        if scroll_view {
            self.scroll_view();
        }

        trace!("{:?}", self.current_buffer().undo);

        next_msg
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

    fn scroll_view(&mut self) {
        let layout = self.layout();
        let cursor_y = self.current_buffer().cursor.y;
        let current_buffer = self.current_buffer_mut();
        if cursor_y < current_buffer.top {
            current_buffer.top = cursor_y as usize;
        } else if cursor_y >= current_buffer.top + layout.buffer.height as usize {
            let diff = cursor_y - (current_buffer.top + layout.buffer.height as usize);
            current_buffer.top += diff as usize + 1;
        }
    }

    fn center_view(&mut self) {
        let layout = self.layout();
        let half_height = layout.buffer.height / 2;
        self.current_buffer_mut().top = self.current_buffer().cursor.y.saturating_sub(half_height as usize);
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
    ToggleMouseCapture,
    DragMouseLeft,
    JumpNextHighlight,
    JumpPreviousHighlight,
    // save under the following name, updating the buffer path
    SaveAs(String),
    // buffer action to undo a paste, removing x graphemes
    UndoPaste(usize),
    UndoBackspace(String),
    UndoDelete(String),
    UndoInsertChar,
    Undo,
    Redo,
}
