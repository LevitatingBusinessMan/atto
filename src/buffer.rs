use std::cmp;

#[derive(Clone, Debug)]
pub struct Buffer {
    pub name: String,
    pub content: Vec<u8>,
    pub position: usize,
    pub read_only: bool,
    /// How far the buffer is scrolled
    pub top: usize,
    /// Which column the cursor wants to be in (that's vague I know)
    pub prefered_col: Option<usize>,
}

impl Buffer {
    pub fn new(name: String, content: Vec<u8>) -> Self {
        return Self {
            name,
            content,
            position: 0,
            read_only: false,
            top: 0,
            prefered_col: None,
        }
    }

    pub fn empty() -> Self {
        return Self {
            name: "Unknown".to_string(),
            content: vec![],
            position: 0,
            read_only: false,
            top: 0,
            prefered_col: None,
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
        for (index, chr) in self.content.iter().enumerate() {
            if index >= self.position {
                break;
            }
            row += 1;
            if *chr == '\n' as u8 {
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
        for (index, chr) in self.content[self.position..].iter().enumerate() {
            if *chr == '\n' as u8 {
                return Some(self.position + index + 1);
            }
        }
        return None;
    }

    fn start_of_line(&self) -> usize {
        for (index, chr) in self.content[..self.position].iter().rev().enumerate() {
            if *chr == '\n' as u8 {
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
        for (index, chr) in self.content[..start_of_line-1].iter().rev().enumerate() {
            if *chr == '\n' as u8 {
                return Some(start_of_line  - 1 - index);
            }
        }
        return Some(0);
    }
}