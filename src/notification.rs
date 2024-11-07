use std::time::{Duration, Instant};

use ratatui::style::Style;

pub struct Notification {
    pub timestamp: Instant,
    pub content: String,
    pub style: Style,
    pub top: usize,
}

impl Notification {
    /// The base duration 
    pub const TIMEOUT_BASE: Duration = Duration::from_millis(1000);
    /// The function that calculates how long a timeout should be
    #[inline]
    fn timeout_fn(content_length: usize) -> Duration {
        // add 10ms per character
        //Duration::from_millis(Self::TIMEOUT_BASE.as_millis() as u64 + content_length as u64 * 10)
        Self::TIMEOUT_BASE
    }

    pub fn new(content: String, style: Style) -> Self {
        tracing::debug!("Notification made with length {} and timeout {}ms", content.len(), Self::timeout_fn(content.len()).as_millis());
        Notification { timestamp: Instant::now(), content, style, top: 0 }
    }

    pub fn expired(&self) -> bool {
        self.timestamp.elapsed() > Self::timeout_fn(self.content.len())
    }

}
