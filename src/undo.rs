use std::time::{Duration, Instant};

use crate::model::Message;

const GROUP_TIME_SPAN: Duration = Duration::new(0, 500_000_000);

/*
 * NOTES
 * I did this the wrong way, if each group is a connected set of actions
 * we can always merge it into a single one.
 *
 * Either a variant of adding without moving the cursor, adding with moving the cursor.
 * Or removing ahead or backwards.
 */

/**
 A group of actions performed in a short time span that
 may be undo'd together.
 I could opt for extending the timespan (tracking latest addition instead of first).
 Still need to test what works best.
*/
#[derive(Debug)]
struct UndoGroup {
    pub start_time: Instant,
    pub messages: Vec<Message>,
    pub inverse: Vec<Message>,
}

impl UndoGroup {
    pub fn new(msg: Message, removed: Option<String>) -> Self {
        let mut messages = vec![];
        let mut inverse = vec![];
        if let Some(inverse_) = invert(&msg, removed) {
            messages.push(msg);
            inverse.push(inverse_);
        }

        Self {
            start_time: Instant::now(),
            messages,
            inverse,
        }
    }
    pub fn still_valid(&self) -> bool {
        self.start_time.elapsed() < GROUP_TIME_SPAN
    }
    pub fn push(&mut self, msg: Message, removed: Option<String>) {
        if let Some(inverse) = invert(&msg, removed) {
            self.messages.push(msg);
            self.inverse.push(inverse);
        }
    }
}

#[derive(Debug)]
pub struct UndoState {
    history: Vec<UndoGroup>,
    /// index of the next group
    index: usize,
}

impl UndoState {
    pub fn new() -> Self {
        Self {
            history: vec![],
            index: 0
        }
    }
    pub fn r#do(&mut self, msg: Message, removed: Option<String>) {
        if !has_inverse(&msg) {
            return;
        }

        self.burn();

        // try to merge with last group
        if let Some(last) = self.previous_group() {
            if last.still_valid() {
                last.push(msg, removed);
                return;
            }
        }

        self.history.push(UndoGroup::new(msg, removed));
        self.index += 1;
    }
    pub fn redo(&mut self) -> Vec<Message> {
        if self.previous_group().is_some() {
            self.index = self.index.saturating_sub(1);
            return self.previous_group().unwrap().inverse.clone();
        } else {
            vec![]
        }
    }
    /// remove any future redo's
    fn burn(&mut self) {
        self.history.truncate(self.index);
    }
    fn previous_group(&mut self) -> Option<&mut UndoGroup> {
        if self.history.len() > 0 && self.index > 0 {
            Some(&mut self.history[self.index-1])
        } else {
            None
        }
    }
}

fn invert(msg: &Message, removed: Option<String>) -> Option<Message> {
    match msg {
        Message::InsertChar(_) => Some(Message::UndoInsertChar),
        Message::Backspace => Some(Message::UndoBackspace(removed.unwrap())),
        Message::Delete => Some(Message::UndoDelete(removed.unwrap())),
        Message::Paste(paste) => Some(Message::UndoPaste(paste.len())),
        _ => None,
    }
}

fn has_inverse(msg: &Message) -> bool {
    matches!(msg, Message::Backspace | Message::Delete | Message::InsertChar(_) | Message::Paste(_))
}
