use std::{cmp, collections::HashMap, fs::File, io::{self, Read, Seek, Stderr, Write}, os::fd::IntoRawFd, process::{self, Stdio}, sync::{Arc, Mutex}, usize};
use syntect::parsing::{SyntaxSet, SyntaxReference};
use tracing::{debug, info};
use unicode_segmentation::UnicodeSegmentation;


use crate::parse::*;

pub static PRIVESC_CMD: &'static str = "run0";

#[derive(Clone, Debug)]
pub struct Buffer {
    pub name: String,
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

impl Buffer {
    pub fn new(name: String, mut file: File, readonly: bool) -> Self {
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        let linestarts = generate_linestarts(&content);
        return Self {
            name,
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

    pub fn textwrap(&mut self, width: usize) {
        self.linestarts.windows(2);
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

    /// update byte position based on the cursor,
    /// assumes the cursor is valid
    pub fn update_position(&mut self) {
        let (start, end) = self.current_line();
        let offset = self.content[start..end].grapheme_indices(true).nth(self.cursor.x).unwrap_or_else(|| (self.current_line_grapheme_length(), "")).0;
        self.position = start + offset;
    }

    /// update cursor based on the byte position
    pub fn update_cursor(&mut self) {
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
    pub fn current_line_grapheme_length(&self) -> usize {
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

    pub fn is_last_line(&self) -> bool {
        self.cursor.y + 2 == self.linestarts.len()
    }

    pub fn is_end_of_line(&self) -> bool {
        let line_len = self.current_line_grapheme_length();
        debug!("ll {}", line_len);
        debug!("isll{}", self.is_last_line());
        if self.is_last_line() && !self.whitespace_terminated() {
            self.cursor.x == line_len
        } else {
            self.cursor.x == line_len.saturating_sub(1)
        }
    }

    pub fn whitespace_terminated(&self) -> bool {
        if let Some(c) = self.content.chars().rev().next() {
            c.is_whitespace()
        } else {
            false
        }
    }
    
    pub fn empty() -> Self {
        return Self {
            name: "".to_string(),
            content: String::new(),
            file: None,
            position: 0,
            cursor: Cursor { x: 0, y: 0 },
            linestarts: vec![0],
            readonly: false,
            opened_readonly: false,
            top: 0,
            prefered_col: None,
            parse_cache: HashMap::new(),
            syntax: None,
            highlights: vec![],
        }
    }

    pub fn set_position(&mut self, pos: usize) {
        self.position = pos;
    }

    pub fn set_readonly(&mut self, ro: bool) {
        self.readonly = ro;
    }

    /// Set the position into the buffer based on a location on the viewport
    pub fn set_viewport_cursor_pos(&mut self, x: u16, y: u16) {
        let mut newlineiter = self.content.chars().enumerate().filter_map(|(i, c)| if c == '\n' || i == 0 {Some(i)} else {None}).skip(self.top + y as usize);
        let mut linestart = newlineiter.next().unwrap_or(0);
        if linestart == 0 { linestart = 0 } else { linestart += 1 };
        self.position = cmp::min(cmp::min(linestart + x as usize, newlineiter.next().unwrap_or(usize::MAX)), self.content.len());
        self.prefered_col = Some(x as usize);
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

    pub fn move_left(&mut self) {
        self.prefered_col = None;
        self.cursor.x = self.cursor.x.saturating_sub(1);
        self.update_position();
    }

    /// move to next grapheme cluster
    pub fn move_right(&mut self) {
        if !self.is_end_of_line() {
            self.prefered_col = None;
            self.cursor.x += 1;
            self.update_position();
        } else if !self.is_last_line() {
            self.prefered_col = Some(0);
            debug!("{:?}", self.prefered_col);
            self.move_down();
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
    

    pub fn move_up(&mut self) {
        let start_of_line = self.start_of_line();
        let prefered_col = self.prefered_col.unwrap_or(self.position.saturating_sub(start_of_line));

        if let Some(start_of_prev_line) = self.start_of_prev_line() {
            let previous_line_length = start_of_line.saturating_sub(start_of_prev_line+1);
            self.position = cmp::min(start_of_prev_line + prefered_col, start_of_prev_line + previous_line_length);
            self.prefered_col = Some(prefered_col);
        } else {
            self.position = start_of_line;
        }
    }

    /// move down a row
    pub fn move_down(&mut self) {
        if self.is_last_line() { return }
        if self.prefered_col.is_none() { self.prefered_col = Some(self.cursor.x); }
        self.cursor.y += 1;
        self.cursor.x = 0;
        let line_length = self.current_line_grapheme_length();
        self.cursor.x = cmp::min(self.prefered_col.unwrap(), line_length.saturating_sub(1));
        self.update_position();
    }

    pub fn page_up(&mut self, height: usize) {
        // let (col, mut row) = self.cursor_pos();
        // row = row.saturating_sub(self.top as u16);
        // self.top = self.top.saturating_sub(height);
        // self.set_viewport_cursor_pos(self.prefered_col.unwrap_or(col as usize) as u16, row);
    }

    pub fn page_down(&mut self, height: usize) {
        // let (col, mut row) = self.cursor_pos();
        // row = row.saturating_sub(self.top as u16);
        // self.top = cmp::min(self.top + height - 1, self.content.lines().count().saturating_sub(height) + 1);
        // self.set_viewport_cursor_pos(self.prefered_col.unwrap_or(col as usize) as u16, row);
    }

    pub fn to_top(&mut self) {
        self.position = 0;
    }

    pub fn to_bottom(&mut self) {
        self.position = self.content.len()-1;
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
    }

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
    }

    pub fn goto_start_of_line(&mut self) {
        self.position = self.start_of_line();
        self.prefered_col = None;
    }

    pub fn goto_end_of_line(&mut self) {
        self.prefered_col = None;
        self.cursor.x = self.current_line_length();
    }

    fn current_char(&self) -> char {
        return self.content.chars().nth(self.position).unwrap();
    }

    pub fn insert(&mut self, chr: char) {
        if !self.readonly {
            self.content.insert(self.position, chr);
            // TODO do not blindly generate linestarts
            self.linestarts = generate_linestarts(&self.content);
            self.move_right();
            // TODO can I invalidate from the current line instead?
            self.parse_cache.invalidate_from(self.top);
        }
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
        let extension = self.name.split('.').last().unwrap_or("");
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
        if self.readonly {
            return Err(io::Error::other("Buffer is readonly"))
        }
        if self.opened_readonly {
            return Err(io::Error::other("No write permission to file"))
        }
        if self.file.is_none() {
            let file = File::options().create(true).write(true).open(self.name.clone())?;
            self.file = Some(Arc::new(Mutex::new(file)));
        }
        let binding = self.file.clone().unwrap();
        let mut file = binding.lock().unwrap();
        file.rewind()?;
        file.write_all(self.content.as_bytes())?;
        file.set_len(self.content.len() as u64)?;

        info!("Wrote {} bytes to {}", self.content.as_bytes().len(), self.name);

        Ok(())
    }

    #[tracing::instrument(skip(self), level="debug")]
    pub fn save_as_root(&mut self) -> io::Result<()> {
        let (reader, mut writer) = std::pipe::pipe()?;
        let mut dd = process::Command::new(PRIVESC_CMD)
            .args(vec!["dd", "bs=4k", &format!("of={}", self.name)])
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
            None => Ok(self.content.is_empty()),
        }
    }

    pub fn paste(&mut self, content: &str) {
        if !self.readonly {
            self.prefered_col = None;
            self.content.insert_str(self.position, content);
            self.position += content.len();
            self.linestarts = generate_linestarts(&self.content);
        }
    }

}

#[test]
fn snowman() {
    let mut buf = Buffer::empty();
    buf.paste("here is ☃ snowman");
    //println!("{:?}", generate_newlines(&buf.content));
    buf.position = 0;
    for _ in 0..12 {
        buf.move_right();
    }
    assert!(buf.position == 12 + String::from("☃").len() - 1);
}

#[test]
fn linestarts() {
    let mut buf = Buffer::empty();
    buf.paste(
"123
123
");
    println!("{:?}", buf.linestarts);
    assert!(buf.linestarts == vec![0,4,8]);
}

#[test]
fn linestarts_snowman() {
    let mut buf = Buffer::empty();
    buf.paste(
"1☃3
123
");
    println!("{:?}", buf.linestarts);
    assert!(buf.linestarts == vec![0,6,10]);
}
