use std::{io::stdout, rc::Rc};

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture}, ExecutableCommand};
use ratatui::{layout::Size, prelude::Backend, style::{Color, Style}};
use syntect::{highlighting::{ThemeSet, Theme}, parsing::SyntaxSet};
use tracing::{debug, error, info, trace, warn};

use crate::{buffer::{self, Buffer}, clipboard::{self, Clipboard}, logging::LogError, themes::colors::notifications::{WARNING_BG, WARNING_FG}, undo::UndoState, utilities::{self, Utility, UtilityWindow, developer::DeveloperModel, save_as::SaveAsModel}};
use crate::notification::Notification;
use crate::themes::colors::notifications::*;

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
    /// did the last message cause an error
    pub last_error: bool,
    pub clipboard: Clipboard,
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

        let clipboard = Clipboard::new();
        debug!("using {clipboard:?} clipboard");

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
            last_error: false,
            clipboard,
        }
    }

    #[tracing::instrument(skip(self), level="debug" fields(last_error=self.last_error))]
    pub fn update(&mut self, msg: Message) {
        // remove notification if elapsed
        // (the handling of this makes it so that if the user does not somehow create a message)
        // the notification will hang around
        if let Some(notification) = &self.notification {
            if notification.expired() {
                self.notification = None;
            }
        }

        let msg = match &mut self.utility {
            Some(UtilityWindow::Find(find)) => find.update(msg),
            Some(UtilityWindow::Help(help)) => help.update(msg),
            Some(UtilityWindow::Confirm(confirm)) => confirm.update(msg),
            Some(UtilityWindow::Developer(developer)) => developer.update(msg),
            Some(UtilityWindow::Shell(shell)) => shell.update(msg),
            Some(UtilityWindow::SaveAs(save_as)) => save_as.update(msg),
            None => Some(msg),
        };

        if let Some(msg) = msg {
            if self.utility.is_some() {
                debug!("Utility {:?} returned {:?}", &self.utility.as_ref().unwrap(), &msg);
            }

            // by default report success
            self.last_error = false;

            // Finally evaluate the message
            self.update_inner(msg);
        }
    }

    fn update_inner(&mut self, msg: Message) {
        match msg {
            Message::NoMessage => {},
            Message::NextBuffer => self.selected = (self.selected + 1) % self.buffers.len(),
            Message::PreviousBuffer => self.selected = (self.selected + self.buffers.len() - 1) % self.buffers.len(),
            Message::QuitNoSave => self.running = false,
            Message::Quit => {
                match self.current_buffer().dirty() {
                    Ok(true) => {
                        if matches!(self.utility, Some(UtilityWindow::SaveAs(_))) {
                            // ignore quit if save as window is open
                            // this is more of a work around
                            // a permanent solution could include variants like SaveQuit, SaveAsQuit and SaveAsRootQuit
                            // messages instead of using a Save + Quit Double
                            // The problem with the Save + Quit double is that when the save files quit is still executed
                            debug!("ignoring quit message, save as utility is open");
                        } else {
                            self.utility = Some(UtilityWindow::Confirm(
                                utilities::confirm::ConfirmModel::new(
                                    String::from("There are unsaved changes. Do you want to save?"),
                                    vec![
                                        ('y', Message::Double(Box::new(Message::Save), Box::new(Message::Quit))),
                                        ('n', Message::QuitNoSave),
                                    ]
                            )));
                        }
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
            Message::Escape => self.update(Message::CloseUtility),
            Message::CloseUtility => self.utility = None,
            Message::InsertChar(chr) => {
                let before = self.current_buffer().position;
                self.current_buffer_mut().insert(chr);
                let after = self.current_buffer().position;
                self.current_buffer_mut().undo.record(before, after, Message::InsertChar(chr), Message::UndoInsertion(1));
                self.scroll_view();
            },
            Message::MoveLeft => {
                self.current_buffer_mut().move_left();
                self.scroll_view();
            },
            Message::MoveRight => {
                self.current_buffer_mut().move_right();
                self.scroll_view();
            },
            Message::MoveUp => {
                self.current_buffer_mut().move_up();
                self.scroll_view();
            },
            Message::MoveDown => {
                self.current_buffer_mut().move_down();
                self.scroll_view();
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
                let before = self.current_buffer().position;
                let removed = self.current_buffer_mut().backspace();
                let after = self.current_buffer().position;
                self.current_buffer_mut().undo.record(before,after, msg, Message::Paste(removed));
            },
            Message::Delete => {
                let before = self.current_buffer().position;
                let removed = self.current_buffer_mut().delete();
                let after = self.current_buffer().position;
                self.current_buffer_mut().undo.record(before, after, msg, Message::InsertString(removed));
            },
            Message::JumpWordLeft => {
                self.current_buffer_mut().move_word_left();
                self.scroll_view();
            },
            Message::JumpWordRight => {
                self.current_buffer_mut().move_word_right();
                self.scroll_view();
            },
            Message::JumpStartOfLine => self.current_buffer_mut().goto_start_of_line(),
            Message::JumpEndOfLine => self.current_buffer_mut().goto_end_of_line(),
            Message::Enter => self.update(Message::InsertChar('\n')),
            Message::Find(query) => {
                let occurences = self.current_buffer_mut().highlight(query);
                // if the find utility is open, set the occurences
                if let Some(UtilityWindow::Find(find)) = &mut self.utility {
                    find.occurences = Some(occurences);
                }
               self.update(Message::JumpNextHighlight);
            },
            Message::Save => {
                if self.current_buffer().name.is_none() {
                    self.utility = Some(UtilityWindow::SaveAs(SaveAsModel::new()));
                } else {
                    if let Err(e) = self.current_buffer_mut().save() {
                        tracing::warn!("{:?}", e);
                        self.update(Message::Notification(
                            format!("{e}"),
                            Style::new().bg(ERROR_BG).fg(ERROR_FG)
                        ));
                        self.last_error = true;
                    } else {
                        self.update(Message::Notification(
                            String::from("Saved"),
                            Style::new().bg(SUCCESS_BG).fg(SUCCES_FG)
                        ));
                    }
                }
            },
            Message::SaveAsRoot => {
                if let Err(e) = self.current_buffer_mut().save_as_root() {
                    tracing::warn!("{e:?}");
                    self.update(Message::Notification(
                        format!("{e}"),
                        Style::new().bg(ERROR_BG).fg(ERROR_FG)
                    ));
                    self.last_error = true;
                } else {
                    self.update(Message::Notification(
                        String::from("Saved as root"),
                        Style::new().bg(WARNING_BG).fg(WARNING_FG)
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
            Message::Paste(ref paste) => {
                let before = self.current_buffer().position;
                self.current_buffer_mut().paste(&paste);
                let after = self.current_buffer().position;
                self.current_buffer_mut().undo.record(before, after, msg.clone(), Message::UndoInsertion(paste.len()));
            },
            Message::PasteClipboard => {
                match self.clipboard.get() {
                    Ok(contents) => {
                        self.update(Message::Paste(contents));
                    },
                    Err(e) => {
                        self.update(Message::Notification(
                            format!("{e}"),
                            Style::new().bg(ERROR_BG).fg(ERROR_FG),
                        ));
                    },
                }
            },
            Message::OpenShell => self.utility = Some(utilities::UtilityWindow::Shell(utilities::shell::ShellModel::new())),
            Message::Double(first, second) => {
                self.update(*first);
                if !self.last_error {
                    info!("yes {:?}", second);
                    self.update(*second);
                } else {
                    warn!("not executing {:?} due to previous error", second);
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
                self.scroll_view();
            },
            Message::ToTop => {
                self.current_buffer_mut().to_top();
                self.scroll_view();
            },
            Message::Tab => {
                self.current_buffer_mut().insert('\t');
                self.scroll_view();
            },
            Message::Suspend => match crate::suspend::suspend().log() {
                Ok(_) => {},
                Err(e) => {
                    let _ = crate::tui::setup();
                    let _ = crate::TERMINAL.get().unwrap().lock().unwrap().clear();
                    self.update(Message::Notification(
                        format!("Suspendin failed with {:?}", e),
                        Style::new().bg(ERROR_BG).fg(ERROR_FG)
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
                self.center_view();
            },
            Message::JumpPreviousHighlight => {
                self.current_buffer_mut().jump_previous_highlight();
                self.center_view();
            },
            Message::SaveAs(path) => {
                let old = self.current_buffer().name.clone();
                self.current_buffer_mut().name = Some(path.clone());
                match self.current_buffer_mut().save() {
                    Ok(()) => {
                        self.update(Message::Notification(
                            format!("Saved as {}", path),
                            Style::new().bg(SUCCESS_BG).fg(SUCCES_FG)
                        ));
                    },
                    Err(e) => {
                        // revert old name/path
                        self.current_buffer_mut().name = old;
                        tracing::warn!("{:?}", e);
                        self.update(Message::Notification(
                            format!("{e}"),
                            Style::new().bg(ERROR_BG).fg(ERROR_FG)
                        ));
                        self.last_error = true;
                    }
                }
            },
            // Message::InsertStringBefore(grapheme)  => {
            //     let at = self.current_buffer().position - grapheme.len();
            //     self.current_buffer_mut().insert_str_at(at, &grapheme);
            // },
            Message::InsertString(string) => {
                self.current_buffer_mut().insert_str(&string);
            },
            Message::Redo => {
                let msgs = self.current_buffer_mut().undo.redo();
                self.update(Message::Many(msgs));
            },
            Message::Undo => {
                let msgs = self.current_buffer_mut().undo.undo();
                self.update(Message::Many(msgs));
            },
            Message::Many(msgs) => {
                for msg in msgs {
                    self.update(msg)
                }
            },
            Message::JumpPosition(position) => {
                self.current_buffer_mut().set_position(position);
                self.center_view();
            },
            Message::InhibitUndo(msg) => {
              self.current_buffer_mut().undo.inhibited = true;
              self.update(*msg);
              self.current_buffer_mut().undo.inhibited = false;
            },
            Message::CutLine => {
                let before = self.current_buffer().position;
                let (start, end) = self.current_buffer().current_line();
                let removed = self.current_buffer_mut().drain(start..end);
                if let Err(e) = self.clipboard.set(removed.clone()) {
                    self.update(Message::Notification(
                        format!("{e}"),
                        Style::new().bg(ERROR_BG).fg(ERROR_FG),
                    ));
                }
                self.current_buffer_mut().set_position(start);
                self.current_buffer_mut().undo.record(before, start, msg, Message::Many(vec![
                    Message::InsertString(removed),
                    Message::JumpPosition(before),
                ]));
            },
            Message::UndoInsertion(n) => {
                let old_position = self.current_buffer().position;
                self.current_buffer_mut().drain(old_position-n..old_position);
                self.current_buffer_mut().set_position(old_position-n);
            },
        };
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

    /// scrolls the current buffer so that the cursor is visible
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

    /// centers the view (on the current buffer)
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
    JumpStartOfLine,
    JumpEndOfLine,
    Enter,
    Save,
    Resize(u16, u16),
    MouseLeft(u16, u16),
    Notification(String, Style),
    DeveloperKey,
    CloseUtility,
    /// Quit immediately
    QuitNoSave,
    /// Writes a string at cursor position.
    /// The user invokes this when using bracketed paste.
    /// For pasting the internal clipboard see [Self::PasteClipboard]
    Paste(String),
    PasteClipboard,
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
    // buffer action to undo an insertation, basically like backspacing n times
    UndoInsertion(usize),
    /// Insert a string **without moving the cursor** (unlike [Message::Paste] or [Message::InsertChar]).
    /// This does not have an undo method and thus should never be constructed outside of redo actions.
    InsertString(String),
    Undo,
    Redo,
    Many(Vec<Message>),
    /// jump to a byte position
    JumpPosition(usize),
    /// run a message while the active buffer's undo is inhibited.
    /// when using this make sure not to use a message that switches the buffer
    InhibitUndo(Box<Message>),
    /// Cut the current line to the clipboad
    CutLine,
}
