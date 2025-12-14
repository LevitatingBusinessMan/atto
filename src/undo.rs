use std::time::{Duration, Instant};

use tracing::{instrument, trace};

use crate::model::Message;

const GROUP_TIME_SPAN: Duration = Duration::new(0, 500_000_000);

/**
 * NOTE
 * I had a new idea that I could use "Absolute" messages for undo/redo.
 * They would just say "insert this at position x", or "delete this range"
 * This would work because the state is always known. Something similar
 * to this would also be necessary for tree-sitter (marking portions of the source as updated).
 * 
 * The current implementation is over-complicated, require a specific "relative"
 * undo action for each do action.
 */

#[derive(Debug)]
/// An action with instructions to reverse it.
pub struct ReversableAction {
    r#do: Message,
    undo: Message,
    position_before: usize,
    position_after: usize,
}

impl ReversableAction {
    pub fn r#do(&self) -> (Message, Message) {
        (Message::JumpPosition(self.position_before), self.r#do.clone())
    }
    pub fn undo(&self) -> (Message, Message) {
        (Message::JumpPosition(self.position_after), self.undo.clone())
    }
}

/**
 A group of actions performed in a short time span that
 may be undo'd together.
 I could opt for extending the timespan (tracking latest addition instead of first).
 Still need to test what works best.
*/
#[derive(Debug)]
struct UndoGroup {
    pub start_time: Instant,
    pub actions: Vec<ReversableAction>,
}

impl UndoGroup {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            actions: vec![],
        }
    }
    pub fn still_valid(&self) -> bool {
        self.start_time.elapsed() < GROUP_TIME_SPAN
    }
    pub fn push(&mut self, position_before: usize, position_after: usize, msg: Message, inverse: Message) {
        self.actions.push(ReversableAction { r#do: msg, undo: inverse, position_before, position_after });
    }
    pub fn r#do(&self) -> Vec<Message> {
        let mut v = Vec::with_capacity(self.actions.len() * 2);
        for action in &self.actions {
            let (jump, msg) = action.r#do();
            v.push(jump);
            v.push(Message::InhibitUndo(Box::new(msg)));
        }
        v
    }
    pub fn undo(&self) -> Vec<Message> {
        let mut v = Vec::with_capacity(self.actions.len() * 2);
        for action in self.actions.iter().rev() {
            let (jump, msg) = action.undo();
            v.push(jump);
            v.push(Message::InhibitUndo(Box::new(msg)));
        }
        v
    }
}

#[derive(Debug)]
pub struct UndoState {
    history: Vec<UndoGroup>,
    /// index of the next group
    index: usize,
    /// if this is set to true, [UndoState::r#do] does nothing
    pub inhibited: bool,
}

impl UndoState {
    pub fn new() -> Self {
        Self {
            history: vec![],
            index: 0,
            inhibited: false,
        }
    }

    #[instrument(skip(self), level="trace", fields(inhibited=self.inhibited))]
    pub fn r#record(&mut self, position_before: usize, position_after: usize, msg: Message, inverse: Message) {
        if self.inhibited {
            return
        }

        self.burn();

        // try to merge with last group
        if let Some(last) = self.previous_group() {
            if last.still_valid() {
                last.push(position_before, position_after, msg, inverse);
                return;
            }
        }

        let mut new_group = UndoGroup::new();
        new_group.push(position_before, position_after, msg, inverse);
        self.history.push(new_group);
        self.index += 1;
    }

    pub fn undo(&mut self) -> Vec<Message> {
        if let Some(prev) = self.previous_group() {
            let msgs = prev.undo();
            let _ = prev;
            self.index = self.index.saturating_sub(1);
            return msgs;
        } else {
            vec![]
        }
    }

    pub fn redo(&mut self) -> Vec<Message> {
        if let Some(next) = self.next_group() {
            let msgs = next.r#do();
            let _ = next;
            self.index += 1;
            return msgs;
        } else {
            vec![]
        }
    }

    /// remove any future redo's
    fn burn(&mut self) {
        trace!("undo stack burned from {}", self.index);
        self.history.truncate(self.index);
    }
    fn previous_group(&mut self) -> Option<&mut UndoGroup> {
        if self.history.len() > 0 && self.index > 0 {
            Some(&mut self.history[self.index-1])
        } else {
            None
        }
    }
    fn next_group(&mut self) -> Option<&mut UndoGroup> {
        if self.history.len() >= self.index+1 {
            Some(&mut self.history[self.index])
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
