//! For manipulating the system and local clipboard

use std::fmt;

use tracing::{debug, error, info, warn};

/// The clipboard used within Atto
pub enum Clipboard {
    System(arboard::Clipboard),
    Local(String),
}

impl fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System(_) => f.debug_tuple("System").finish(),
            Self::Local(arg0) => f.debug_tuple("Local").field(arg0).finish(),
        }
    }
}

#[derive(Debug)]
pub struct Error(arboard::Error);

impl Clipboard {
    pub fn new() -> Self {
        match arboard::Clipboard::new() {
            Ok(clipboard) => Self::System(clipboard),
            Err(e) => {
                warn!("{e:?}");
                Self::Local(String::new())
            },
        }
    }
    pub fn get(&mut self) -> Result<String, Error> {
        match self {
            Clipboard::System(clipboard) => match clipboard.get().text() {
                Ok(contents) => Ok(contents),
                Err(e) => {
                    warn!("{e:?}");
                    Err(Error(e))
                },
            },
            Clipboard::Local(contents) => Ok(contents.clone()),
        }
    }
    pub fn set(&mut self, content: String) -> Result<(), Error> {
        let length = content.len();
        match self {
            Clipboard::System(clipboard) => match clipboard.set_text(content) {
                Ok(()) => {
                    info!("Saved {} bytes to system clipboard", length);
                    Ok(())
                },
                Err(e) => {
                    error!("{e:?}");
                    Err(Error(e))
                },
            },
            Clipboard::Local(string) => {
                *string = content;
                debug!("Saved {} bytes to local clipboard", length);
                Ok(())
            },
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            arboard::Error::ContentNotAvailable => f.write_str("clipboard unavailable"),
			arboard::Error::ClipboardNotSupported => f.write_str("clipboard content not supported"),
			arboard::Error::ClipboardOccupied => f.write_str("clipboard occupation error"),
			arboard::Error::ConversionFailure => f.write_str("clipboard utf-8 conversion error"),
			arboard::Error::Unknown { ref description } => f.write_fmt(format_args!("unknown clipboard error '{description}'")),
			_ => f.write_str("unknown clipboard error"),
        }
    }
}
