use std::{cmp, collections::HashMap};
use syntect::parsing::{SyntaxDefinition, SyntaxSet, SyntaxReference};
use anyhow::anyhow;

use crate::parse::*;

#[derive(Clone, Debug)]
pub struct Buffer {
    pub name: String,
    pub content: String,
    pub position: usize,
    pub read_only: bool,
    /// How far the buffer is scrolled
    pub top: usize,
    /// Which column the cursor wants to be in (that's vague I know)
    pub prefered_col: Option<usize>,
    /// The cached parse states for this buffer
    pub parse_cache: HashMap<usize, CachedParseState>,
    pub syntax: Option<SyntaxReference>,
}

impl Buffer {
    pub fn new(name: String, content: String) -> Self {
        return Self {
            name,
            content,
            position: 0,
            read_only: false,
            top: 0,
            prefered_col: None,
            parse_cache: HashMap::new(),
            syntax: None,
        }
    }

    pub fn empty() -> Self {
        return Self {
            name: "Unknown".to_string(),
            content: String::new(),
            position: 0,
            read_only: false,
            top: 0,
            prefered_col: None,
            parse_cache: HashMap::new(),
            syntax: None,
        }
    }

    pub fn set_position(&mut self, pos: usize) {
        self.position = pos;
    }

    pub fn set_readonly(&mut self, ro: bool) {
        self.read_only = ro;
    }

    /// Get position as column and row
    pub fn cursor_pos(&self) -> (u16, u16) {
        let mut newlines = 0;
        let mut row = 0;
        for (index, chr) in self.content.chars().enumerate() {
            if index >= self.position {
                break;
            }
            row += 1;
            if chr == '\n' {
                newlines += 1;
                row = 0;
            }
        }
        return (row, newlines)
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

    pub fn move_word_right(&mut self) {
        if self.current_char().is_whitespace() {
            while self.current_char().is_whitespace() && self.position+1 != self.content.len() {
                self.position += 1;
            }
        } else if self.current_char().is_alphanumeric() {
            while self.current_char().is_alphanumeric() && self.position+1 != self.content.len() {
                self.position += 1;
            }
        } else {
            while !self.current_char().is_alphanumeric() && self.position+1 != self.content.len() {
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

}