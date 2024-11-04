use std::{cmp, collections::HashMap, fs::File, io::{self, Read, Seek, Write}, sync::{Arc, Mutex}, usize};
use ratatui::symbols::line;
use syntect::parsing::{SyntaxSet, SyntaxReference};
use tracing::{debug, info};

use crate::parse::*;

#[derive(Clone, Debug)]
pub struct Buffer {
    pub name: String,
    pub content: String,
    pub file: Option<Arc<Mutex<File>>>,
    pub position: usize,
    pub read_only: bool,
    /// How far the buffer is scrolled
    pub top: usize,
    /// Which column the cursor wants to be in (that's vague I know)
    pub prefered_col: Option<usize>,
    /// The cached parse states for this buffer
    pub parse_cache: HashMap<usize, CachedParseState>,
    pub syntax: Option<SyntaxReference>,
    pub highlights: Vec<(usize, usize)>,
}

impl Buffer {
    pub fn new(name: String, mut file: File) -> Self {
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        return Self {
            name,
            content: content,
            file: Some(Arc::new(Mutex::new(file))),
            position: 0,
            read_only: false,
            top: 0,
            prefered_col: None,
            parse_cache: HashMap::new(),
            syntax: None,
            highlights: vec![],
        }
    }

    pub fn empty() -> Self {
        return Self {
            name: "Unknown".to_string(),
            content: String::new(),
            file: None,
            position: 0,
            read_only: false,
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
        self.read_only = ro;
    }

    /// Set the position into the buffer based on a location on the viewport
    pub fn set_viewport_cursor_pos(&mut self, x: u16, y: u16) {
        let mut newlineiter = self.content.chars().enumerate().filter_map(|(i, c)| if c == '\n' || i == 0 {Some(i)} else {None}).skip(self.top + y as usize);
        let mut linestart = newlineiter.next().unwrap_or(0);
        if linestart == 0 { linestart = 0 } else { linestart += 1 };
        self.position = cmp::min(cmp::min(linestart + x as usize, newlineiter.next().unwrap_or(usize::MAX)), self.content.len());
    }

    /// Get position as column and row (of the total buffer not the viewport)
    pub fn cursor_pos(&self) -> (u16, u16) {
        let mut row = 0;
        let mut col = 0;
        for (index, chr) in self.content.chars().enumerate() {
            if index >= self.position {
                break;
            }
            if chr == '\t' {
                col += crate::parse::TABSIZE as u16;
            } else {
                col += 1;
            }
            if chr == '\n' {
                row += 1;
                col = 0;
            }
        }
        return (col, row)
    }

    pub fn move_left(&mut self) {
        self.prefered_col = None;
        self.position = self.position.saturating_sub(1);
    }
    
    pub fn move_right(&mut self) {
        self.prefered_col = None;
        self.position = cmp::min(self.position + 1, self.content.len());
    }
    
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
    
    pub fn move_down(&mut self) {
        let prefered_col = self.prefered_col.unwrap_or(self.position.saturating_sub(self.start_of_line()));
        if let Some(start_of_next_line) = self.start_of_next_line() {
            self.position = start_of_next_line;
            let start_of_next_next_line = self.start_of_next_line().unwrap_or(self.content.len());
            let next_line_length = start_of_next_next_line.saturating_sub(start_of_next_line + 1);
            self.position = cmp::min(start_of_next_line + prefered_col, start_of_next_line + next_line_length);
            self.prefered_col = Some(prefered_col);
        } else {
            self.position = self.content.len();
        }
    }

    pub fn page_up(&mut self, height: usize) {
        self.top = self.top.saturating_sub(height);
    }
    
    pub fn page_down(&mut self, height: usize) {
        self.top = cmp::min(self.top + height - 1, self.content.lines().count() - height);
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
        self.position = match self.start_of_next_line() {
            Some(start_of_next_line) => start_of_next_line - 1,
            None => self.content.len(),
        };
        self.prefered_col = None;
    }

    fn current_char(&self) -> char {
        return self.content.chars().nth(self.position).unwrap();
    }

    pub fn insert(&mut self, chr: char) {
        self.content.insert(self.position, chr);
        self.move_right();
        // invalidating from top is faster than figuring out the current line
        // and you render from the top anyway
        self.parse_cache.invalidate_from(self.top);
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
        if self.file.is_none() {
            let file = File::options().create(true).write(true).open(self.name.clone())?;
            self.file = Some(Arc::new(Mutex::new(file)));
        }
        let binding = self.file.clone().unwrap();
        let mut file = binding.lock().unwrap();
        file.rewind()?;
        file.write_all(self.content.as_bytes())?;
        file.flush()?;

        info!("Wrote {} bytes to {}", self.content.as_bytes().len(), self.name);

        Ok(())
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

}