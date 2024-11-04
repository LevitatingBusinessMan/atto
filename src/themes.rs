use std::io::{self, Cursor};

use syntect::highlighting::ThemeSet;

static DRACULA: &[u8] =  include_bytes!("../themes/Dracula.tmTheme");

pub fn theme_set() -> io::Result<ThemeSet> {
    let mut theme_set = ThemeSet::load_defaults();

    let dracula = ThemeSet::load_from_reader(&mut Cursor::new(DRACULA))
    .map_err(|_| io::ErrorKind::InvalidData)?;

    theme_set.themes.insert("dracula".to_owned(), dracula);

    Ok(theme_set)
}
