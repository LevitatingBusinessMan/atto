use std::io::{self, Cursor};

use syntect::highlighting::ThemeSet;

static DRACULA: &[u8] =  include_bytes!("../themes/Dracula.tmTheme");

pub mod colors {
    pub mod notifications {
        use ratatui::style::Color;
        pub const ERROR_BG: Color =  Color::Rgb(200, 0, 0);
        pub const ERROR_FG: Color =  Color::White;
        pub const SUCCESS_BG: Color = Color::Rgb(0, 180, 0);
        pub const SUCCES_FG: Color = Color::White;
        pub const WARNING_BG: Color = Color::Yellow;
        pub const WARNING_FG: Color = Color::White;
    }
}

pub fn theme_set() -> io::Result<ThemeSet> {
    let mut theme_set = ThemeSet::load_defaults();

    let dracula = ThemeSet::load_from_reader(&mut Cursor::new(DRACULA))
    .map_err(|_| io::ErrorKind::InvalidData)?;

    theme_set.themes.insert("dracula".to_owned(), dracula);

    Ok(theme_set)
}
