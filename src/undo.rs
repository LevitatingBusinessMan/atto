use std::time::{Duration, Instant};

use crate::model::Message;

const GROUP_TIME_SPAN: Duration = Duration::new(0, 500_000_000);

/**
 * NOTES
 *
 * For undo it is useful to be able to execute many messages to the model with ease.
 * That's why I think it might be useful to have a Message:Many message.
 * I could also then split the update function to have an inner version which doesn't do the view related work.
 *
 * All the do and undo commands require a position.
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
    pub fn new(msg: Message, inverse: Message) -> Self {
        let mut messages = vec![msg];
        let mut inverse = vec![inverse];

        Self {
            start_time: Instant::now(),
            messages,
            inverse,
        }
    }
    pub fn still_valid(&self) -> bool {
        self.start_time.elapsed() < GROUP_TIME_SPAN
    }
    pub fn push(&mut self, msg: Message, inverse: Message) {
        self.messages.push(msg);
        self.inverse.push(inverse);
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
    pub fn r#do(&mut self, msg: Message, inverse: Message) {
        self.burn();

        // try to merge with last group
        if let Some(last) = self.previous_group() {
            if last.still_valid() {
                last.push(msg, inverse);
                return;
            }
        }

        self.history.push(UndoGroup::new(msg, inverse));
        self.index += 1;
    }

    pub fn undo(&mut self) -> Vec<Message> {
        if self.previous_group().is_some() {
            let inverse_msgs = self.previous_group().unwrap().inverse.clone();
            self.index = self.index.saturating_sub(1);
            return inverse_msgs.into_iter().rev().collect()
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

// fn invert(msg: &Message, removed: Option<String>) -> Option<Message> {
//     match msg {
//         Message::InsertChar(_) => Some(Message::UndoInsertChar),
//         Message::Backspace => Some(Message::UndoBackspace(removed.unwrap())),
//         Message::Delete => Some(Message::UndoDelete(removed.unwrap())),
//         Message::Paste(paste) => Some(Message::UndoPaste(paste.len())),
//         _ => None,
//     }
// }

// fn has_inverse(msg: &Message) -> bool {
//     matches!(msg, Message::Backspace | Message::Delete | Message::InsertChar(_) | Message::Paste(_))
// }
