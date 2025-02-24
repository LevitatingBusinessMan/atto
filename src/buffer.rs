use std::{cmp, collections::HashMap, fs::File, io::{self, Read, Seek, Stderr, Write}, os::fd::IntoRawFd, process::{self, Stdio}, sync::{Arc, Mutex}, usize};
use syntect::parsing::{SyntaxSet, SyntaxReference};
use tracing::{debug, info};
use unicode_segmentation::{GraphemeCursor, GraphemeIndices, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;


use crate::{logging::LogError, parse::*};

pub static PRIVESC_CMD: &'static str = "run0";

#[derive(Clone, Debug)]
pub struct Buffer {
    pub name: Option<String>,
    pub content: String,
    pub file: Option<Arc<Mutex<File>>>,
	/// cursors byte index into the buffer
    pub position: usize,
	/// visual (grapheme) cursor position
	pub cursor: Cursor,
	/// the indexes of all the beginnings of lines
	pub linestarts: Vec<usize>,
    /// the file was opened as readonly
    pub opened_readonly: bool,
    /// This buffer shall not be edited
    pub readonly: bool,
    /// How far the buffer is scrolled
    pub top: usize,
    /// Which column the cursor wants to be in (that's vague I know)
    pub prefered_col: Option<usize>,
    /// The cached parse states for this buffer
    pub parse_cache: HashMap<usize, CachedParseState>,
    pub syntax: Option<SyntaxReference>,
    pub highlights: Vec<(usize, usize)>,
}

fn generate_linestarts(content: &str) -> Vec<usize> {
    let mut ns: Vec<usize> = vec![0];
    ns.extend(content.bytes().enumerate().filter_map(|(i, b)| if b == b'\n' {Some(i+1)} else {None}));
    //if content.chars().last().is_some_and(|c| c != '\n') { ns.push(content.len()) }
    ns.push(content.len());
    ns
}

// pub fn generate_linestarts_textwrap(content: &str, width: usize) -> Vec<usize> {
//     let mut ns: Vec<usize> = vec![0];
//     ns.extend(content.bytes().enumerate().filter_map(|(i, b)| if b == b'\n' {Some(i+1)} else {None}));
//     //if content.chars().last().is_some_and(|c| c != '\n') { ns.push(content.len()) }
//     ns.push(content.len());
//     ns
// }

//* The column and line of the cursor, starting at (0,0) */
#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub x: usize,
    pub y: usize,
}

/// how much columns to use for this grapheme cluster
/// TODO should I really consider newlines not to take a column?
/// considering they can be rendered with a column
// pub fn grapheme_width(gr: &str) -> usize {
//     if gr.chars().any(|c| c == '\t') {
//         crate::parse::whitespace::TABSIZE
//     } else {
//         gr.width()
//     }
// }

/// the amount of columsn a str will take,
/// so grapheme clusters plus tab slots
pub fn str_column_length(s: &str) -> usize {
    perform_str_replacements(s, false).width()
}

/// like [str_column_length] but it strips the newline at the end
pub fn str_column_length_no_lb(s: &str) -> usize {
    str_column_length(s.trim_end_matches(|c| c == '\r'|| c == '\n'))
}

impl Buffer {
    pub fn new(name: String, mut file: File, readonly: bool) -> Self {
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        let linestarts = generate_linestarts(&content);
        return Self {
            name: Some(name),
            content: content,
            file: Some(Arc::new(Mutex::new(file))),
            position: 0,
            cursor: Cursor { x: 0, y: 0 },
            linestarts,
            readonly: false,
            opened_readonly: readonly,
            top: 0,
            prefered_col: None,
            parse_cache: HashMap::new(),
            syntax: None,
            highlights: vec![],
        }
    }

    pub fn empty() -> Self {
        return Self {
            name: None,
            content: String::new(),
            file: None,
            position: 0,
            cursor: Cursor { x: 0, y: 0 },
            linestarts: generate_linestarts(""),
            readonly: false,
            opened_readonly: false,
            top: 0,
            prefered_col: None,
            parse_cache: HashMap::new(),
            syntax: None,
            highlights: vec![],
        }
    }

    pub fn increase_all_linestarts(&mut self, from: usize, n: usize) {
        self.linestarts.iter_mut().for_each(|ls| if from >= *ls { *ls = ls.saturating_add(n) });
    }

    /// awful bug fix for a dumb design flaw.
    /// gets the amount of excess bytes preceding
    /// the position due to multi-byte graphemes
    pub fn magic_unicode_offset_bug_fix(&self) -> usize {
         self.content.grapheme_indices(true)
            .filter(|(i, s)| i < &self.position && s.len() > 1)
            .fold(0, |a, (_i, s)| a + s.len() - 1);
        return 0
    }

	/// number of excess bytes between two points caused
	/// by multi-byte graphemes
    pub fn excess_bytes(&self, start: usize, end: usize) -> usize {
        let chunk = &self.content[start..end];
        return chunk.len() - chunk.graphemes(true).count();
    }

    /// update byte position based on the cursor
    /// the cursor must be within bounds, but if it sits on a tab (or similar)
    /// it will move to the right
    pub fn update_position(&mut self) {
        let line_graphemes: Vec<&str> = self.current_line_str().graphemes(true).collect();
        let mut pos = 0;
        let mut col = 0;
        for gr in line_graphemes {
            if col >= self.cursor.x {
                self.cursor.x = col;
                break;
            }
            col += str_column_length(gr);
            pos += gr.len();
        }
        self.position = self.linestarts[self.cursor.y] + pos;
    }

    /// update cursor based on the byte position
    pub fn update_cursor(&mut self) {
        for (i, win) in self.linestarts.windows(2).enumerate() {
            if win[1] > self.position {
                self.cursor.y = i;
                self.cursor.x = str_column_length(self.current_line_str_before_cursor());
                return;
            }
        }
        // especial last line handling
        self.cursor.y = self.linestarts.len() - 2;
        self.cursor.x = str_column_length(self.current_line_str_before_cursor());
    }


    /// current line start and end using only self.cursor.y
    pub fn current_line(&self) -> (usize, usize) {
        return (self.linestarts[self.cursor.y], self.linestarts[self.cursor.y+1]);
    }

    /// length of current line in bytes
    pub fn current_line_length(&self) -> usize {
        let (start, end) = self.current_line();
        end - start
    }

    /// length of current line in grapheme clusters
    pub fn current_line_grapheme_count(&self) -> usize {
        self.current_line_str().graphemes(true).count()
    }

    /// length of current line in grapheme clusters excluding linebreak
    // pub fn current_line_grapheme_length_no_lb(&self) -> usize {
    //     let str = self.current_line_str();
    //     if str.graph
    // }

    pub fn current_line_str(&self) -> &str {
        &self.content[self.linestarts[self.cursor.y]..self.linestarts[self.cursor.y+1]]
    }

    /// this one use self.position, so do not use it to calculate the position (please)
    pub fn current_line_str_before_cursor(&self) -> &str {
        &self.content[self.linestarts[self.cursor.y]..self.position]
    }

    pub fn is_first_line(&self) -> bool {
        self.cursor.y == 0
    }

    pub fn is_last_line(&self) -> bool {
        self.cursor.y + 2 == self.linestarts.len()
    }

    pub fn set_position(&mut self, pos: usize) {
        self.position = pos;
    }

    pub fn set_readonly(&mut self, ro: bool) {
        self.readonly = ro;
    }

    /// Set the position into the buffer based on a location on the viewport
    pub fn set_viewport_cursor_pos(&mut self, x: u16, y: u16) {
        self.cursor.y = cmp::min(self.top + y as usize, self.linestarts.len() - 2);
        self.prefered_col = Some(x as usize);
        self.place_cursor_x(x as usize);
        self.update_position();
    }

    /// Get position as column and row (of the total buffer not the viewport)
    // pub fn cursor_pos(&self) -> (u16, u16) {
    //     let mut row = 0;
    //     let mut col = 0;
    //     for (index, chr) in self.content.chars().enumerate() {
    //         if index >= self.position {
    //             break;
    //         }
    //         if chr == '\t' {
    //             col += crate::parse::whitespace::TABSIZE as u16;
    //         } else {
    //             col += 1;
    //         }
    //         if chr == '\n' {
    //             row += 1;
    //             col = 0;
    //         }
    //     }
    //     return (col, row)
    // }

    /// return the previous grapheme string and its left boundary
    pub fn prev_grapheme(&self) -> Option<(&str, usize)> {
        let mut gcursor = GraphemeCursor::new(self.position, self.content.len(), true);
        match gcursor.prev_boundary(&self.content, 0).log() {
            Ok(Some(pb)) => {
                Some((&self.content[pb..self.position], pb))
            },
            Ok(None) | Err(_) => None,
        }
    }

    /// return the previous grapheme string and its right boundary
    pub fn cur_grapheme(&self) -> Option<(&str, usize)> {
        let mut gcursor = GraphemeCursor::new(self.position, self.content.len(), true);
        match gcursor.next_boundary(&self.content, 0).log() {
            Ok(Some(pb)) => {
                Some((&self.content[self.position..pb], pb))
            },
            Ok(None) | Err(_) => None,
        }
    }
    /// move left to previous grapheme cluster
    pub fn move_left(&mut self) {
        if let Some((_s, i)) = self.prev_grapheme() {
            self.position = i;
            self.prefered_col = None;
            self.update_cursor();
        }
    }

    /// move to next grapheme cluster
    pub fn move_right(&mut self) {
        if let Some((_s, b)) = self.cur_grapheme() {
            self.position = b;
            self.prefered_col = None;
            self.update_cursor();
        }
    }

    // OLD cursor based move_right behaviour
    // /// move a column to the right
    // pub fn move_right(&mut self) {
    //     if !self.is_end_of_line() {
    //         self.prefered_col = None;
    //         self.cursor.x += 1;
    //         self.update_position();
    //     } else if !self.is_last_line() {
    //         self.prefered_col = Some(0);
    //         self.move_down();
    //     } 
    // }
    

    /// place the x cursor anywhere on the line,
    /// assuming cursor.y is set this will move it to position or eol
    /// and handle the preferred_col
    pub fn place_cursor_x(&mut self, x: usize) {
        let line_length = str_column_length_no_lb(self.current_line_str());
        self.prefered_col = Some(self.prefered_col.unwrap_or(x));
        self.cursor.x = cmp::min(self.prefered_col.unwrap(), line_length);
    }

    /// move up a row
    pub fn move_up(&mut self) {
        if self.is_first_line() { self.goto_start_of_line(); return }
        self.cursor.y = self.cursor.y.saturating_sub(1);
        self.place_cursor_x(self.cursor.x);
        self.update_position();
    }

    /// move down a row
    pub fn move_down(&mut self) {
        if self.is_last_line() { self.goto_end_of_line(); return }
        self.cursor.y += 1;
        self.place_cursor_x(self.cursor.x);
        self.update_position();
    }

    pub fn page_up(&mut self, height: usize) {
        self.top = self.top.saturating_sub(height);
        self.cursor.y = self.cursor.y.saturating_sub(height);
        self.place_cursor_x(self.cursor.x);
        self.update_position();
    }

    pub fn page_down(&mut self, height: usize) {
        self.top = cmp::min(self.top + height, self.linestarts.len() - height);
        self.cursor.y = cmp::min(self.cursor.y + height, self.linestarts.len() - 2);
        self.place_cursor_x(self.cursor.x);
        self.update_position();
    }

    pub fn to_top(&mut self) {
        self.position = 0;
        self.update_cursor();
    }

    pub fn to_bottom(&mut self) {
        self.position = self.content.len()-1;
        self.update_cursor();
    }

    fn start_of_next_line(&self) -> Option<usize> {
        for (index, chr) in self.content[self.position..].chars().enumerate() {
            if chr == '\n' {
                return Some(self.position + index + 1);
            }
        }
        return None;
    }

    fn start_of_line(&self) -> usize {
        for (index, chr) in self.content[..self.position].chars().rev().enumerate() {
            if chr == '\n' {
                return self.position - index;
            }
        }
        return 0;
    }

    fn start_of_prev_line(&self) -> Option<usize> {
        let start_of_line = self.start_of_line();
        if start_of_line == 0 {
            return None;
        }
        for (index, chr) in self.content[..start_of_line-1].chars().rev().enumerate() {
            if chr == '\n' {
                return Some(start_of_line  - 1 - index);
            }
        }
        return Some(0);
    }

    // TODO rewrite to match new utilities
    pub fn move_word_left(&mut self) {
        let mut next = self.content.chars().nth(self.position.saturating_sub(1)).unwrap();
        if next.is_whitespace() {
            while next.is_whitespace() && self.position > 0 && self.start_of_line() != self.position {
                self.position -= 1;
                next = self.content.chars().nth(self.position.saturating_sub(1)).unwrap();
            }
        } else if next.is_alphanumeric() {
            while (next.is_alphanumeric() || next == '_') && self.position > 0 && self.start_of_line() != self.position {
                self.position -= 1;
                next = self.content.chars().nth(self.position.saturating_sub(1)).unwrap();
            }
        } else {
            while !next.is_alphanumeric()  && !next.is_whitespace() && self.position > 0 && self.start_of_line() != self.position {
                self.position -= 1;
                next = self.content.chars().nth(self.position.saturating_sub(1)).unwrap();
            }
        }
        self.prefered_col = None;
        self.update_cursor();
    }

    // TODO rewrite to match new utilities
    pub fn move_word_right(&mut self) {
        if self.current_char().is_whitespace() {
            while self.current_char().is_whitespace() && self.position+1 != self.content.len() && self.current_char() != '\n' {
                self.position += 1;
            }
        } else if self.current_char().is_alphanumeric() {
            while (self.current_char().is_alphanumeric() || self.current_char() == '_') && self.position+1 != self.content.len() {
                self.position += 1;
            }
        } else {
            while !self.current_char().is_alphanumeric()  && !self.current_char().is_whitespace() && self.position+1 != self.content.len() {
                self.position += 1;
            }
        }
        self.prefered_col = None;
        self.update_cursor();;
    }

    pub fn goto_start_of_line(&mut self) {
        self.cursor.x = 0;
        self.prefered_col = None;
        self.update_position();
    }

    pub fn goto_end_of_line(&mut self) {
        self.cursor.x = str_column_length_no_lb(self.current_line_str());
        self.prefered_col = None;
        self.update_position();
    }

    fn current_char(&self) -> char {
        return self.content.chars().nth(self.position).unwrap();
    }

    pub fn find(&mut self, query: String) {
        let matches: Vec<_> = self.content.match_indices(&query).map(|(start, match_)| {
            (start, start + match_.len())
        }).collect();

        // scroll to first match
        if let Some((start, _end)) = matches.iter().find(|(start, _end)| start >= &self.position) {
            self.position = *start
        }

        self.highlights = matches;
    }

    // Tries to find and set a syntax
    pub fn find_syntax<'a>(&mut self, syntax_set: &'a SyntaxSet) -> Option<&'a SyntaxReference> {
        let name = self.name.clone()?;
        let extension = name.split('.').last().unwrap_or("");
        let syntax = match syntax_set.find_syntax_by_extension(extension) {
            Some(syntax) => Some(syntax),
            None => {
                match self.content.lines().next() {
                    Some(first_line) => syntax_set.find_syntax_by_first_line(&first_line),
                    None => None,
                }
            },
        };
        if let Some(syntax) = syntax {
            self.syntax = Some(syntax.clone());
        }
        syntax
    }

    /// save to disk
    pub fn save(&mut self) -> io::Result<()> {
        if self.name.is_none() {
            return Err(io::Error::new(io::ErrorKind::Other, "no path specified"));
        }
        if self.readonly {
            return Err(io::Error::other("Buffer is readonly"))
        }
        if self.opened_readonly {
            return Err(io::Error::other("No write permission to file"))
        }
        if self.file.is_none() {
            let file = File::options().create(true).read(true).write(true).open(self.name.clone().unwrap())?;
            self.file = Some(Arc::new(Mutex::new(file)));
        }
        let binding = self.file.clone().unwrap();
        let mut file = binding.lock().unwrap();
        file.rewind()?;
        file.write_all(self.content.as_bytes())?;
        file.set_len(self.content.len() as u64)?;

        info!("Wrote {} bytes to {}", self.content.as_bytes().len(), self.name.clone().unwrap());

        Ok(())
    }

    #[tracing::instrument(skip(self), level="debug")]
    pub fn save_as_root(&mut self) -> io::Result<()> {
        if self.name.is_none() {
            return Err(io::Error::new(io::ErrorKind::Other, "no path specified"));
        }
        let (reader, mut writer) = io::pipe()?;
        let mut dd = process::Command::new(PRIVESC_CMD)
            .args(vec!["dd", "bs=4k", &format!("of={}", self.name.clone().unwrap())])
            .stdin(reader)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        writer.write_all(self.content.as_bytes())?;
        writer.flush()?;
        nix::unistd::close(writer.into_raw_fd())?;
        let status = dd.wait()?;
        match status.success() {
            true => Ok(()),
            false => {
                let mut stderr = String::new();
                dd.stderr.unwrap().read_to_string(&mut stderr)?;
                Err(io::Error::other(stderr))
            },
        }
    }

    pub fn dirty(&self) -> io::Result<bool> {
        match &self.file {
            Some(file) => {
                let mut filecontent = String::new();
                let mut file = file.lock().unwrap();
                file.rewind()?;
                file.read_to_string(&mut filecontent)?;
                Ok(filecontent != self.content)
            },
            None => Ok(true),
        }
    }

    // read only should be handled in model

    pub fn insert(&mut self, chr: char) {
        self.content.insert(self.position, chr);
        self.position += 1;
        // TODO do not blindly generate linestarts
        self.linestarts = generate_linestarts(&self.content);
        self.update_cursor();
        // TODO can I invalidate from the current line instead?
        self.parse_cache.invalidate_from(self.top);
    }

    pub fn paste(&mut self, content: &str) {
        self.prefered_col = None;
        self.content.insert_str(self.position, content);
        self.position += content.len();
        self.linestarts = generate_linestarts(&self.content);
    }

    pub fn backspace(&mut self) {
        if let Some((s, b)) = self.prev_grapheme() {
            self.content.drain(b..self.position);
            self.position = b;
            self.prefered_col = None;
            self.linestarts = generate_linestarts(&self.content);
            self.update_cursor();
        }
    }

    pub fn delete(&mut self) {
        if let Some((_s, b)) = self.cur_grapheme() {
            self.content.drain(self.position..b);
            self.linestarts = generate_linestarts(&self.content);
            self.update_cursor();
        }
    }

}

#[test]
fn snowman() {
    let mut buf = Buffer::empty();
    buf.paste("here is ☃ snowman");
    buf.position = 0;
    for _ in 0..12 {
        buf.move_right();
    }
    assert!(buf.position == 12 + String::from("☃").len() - 1);
}

#[test]
fn step_over_y() {
    let mut buf = Buffer::empty();
    let y = "y̆";
    buf.paste(y);
    buf.position = 0;
    buf.move_right();
    assert!(buf.position == 3);
}

#[test]
fn step_over_flags() {
    let mut buf = Buffer::empty();
    let flags: &str = "🇷🇺🇸🇹";
    buf.paste(flags);
    buf.position = 0;
    buf.move_right();
    assert!(buf.position == 8);
    assert!(buf.cursor.x == 2);
}

#[test]
fn step_over_ghosts() {
    let mut buf = Buffer::empty();
    let ghosts: &str = "👻👻👻";
    buf.paste(ghosts);
    buf.position = 0;
    buf.move_right();
    assert!(buf.position == 4);
    assert!(buf.cursor.x == 2);
}

#[test]
fn linestarts() {
    let mut buf = Buffer::empty();
    buf.paste(
"123
123
");
    println!("{:?}", buf.linestarts);
    assert!(buf.linestarts == vec![0,4,8,8]);
}

#[test]
fn linestarts_snowman() {
    let mut buf = Buffer::empty();
    buf.paste(
"1☃3
123
");
    println!("{:?}", buf.linestarts);
    assert!(buf.linestarts == vec![0,6,10,10]);
}

#[test]
fn linestarts_no_lb() {
    let mut buf = Buffer::empty();
    buf.paste(
"123");
    println!("{:?}", buf.linestarts);
    assert!(buf.linestarts == vec![0,3]);
}

#[test]
fn linestarts_empty() {
    let buf = Buffer::empty();
    println!("{:?}", buf.linestarts);
    assert!(buf.linestarts == vec![0,0]);
}

