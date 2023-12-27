use std::cell::LazyCell;

use ratatui::text::Line;
use syntect::highlighting::{ThemeSet, HighlightState, HighlightIterator, Highlighter, Theme};
use syntect::parsing::{SyntaxSet, ParseState, ScopeStack, SyntaxReference};
use syntect::util::LinesWithEndings;
use syntect::highlighting::Style as SyntectStyle;
use anyhow::anyhow;
use ratatui::text::Span;
use ratatui::style::Stylize;

use super::Buffer;

#[derive(Clone, Debug)]
pub struct HighLightCache<'a> {
    pub lines: Vec<Line<'a>>,
    pub dirty: bool,
}

// #[derive(Clone, Debug)]
// pub struct LineCache<'a> {
//     pub highlights: Line<'a>,
// }

// TODO this will have to move
// preferably, after an insert, a thread is run
// which uses a RefCell to update the highlighted lines
// of the buffer. Preferably with some caching, maybe even with only parsing the edited lines.
// The view should use this if it can be borrowed.
// We can use the scopes for navigation (in HighlightState).

// See https://docs.rs/syntect/latest/syntect/highlighting/struct.HighlightState.html
// https://docs.rs/syntect/latest/syntect/parsing/struct.ParseState.html

pub fn highlight<'a>(ss: &SyntaxSet, theme: &Theme, syntax: &SyntaxReference, buffer: &'a str) -> anyhow::Result<HighLightCache<'a>> {
    let lines = LinesWithEndings::from(buffer);

    let hl = Highlighter::new(theme);
    let mut ps = ParseState::new(syntax);
    let mut hs = HighlightState::new(&hl, ScopeStack::new());

    let mut token_lines = vec![];

    for line in lines {
        let ops = ps.parse_line(line, &ss)?;
        let iter = HighlightIterator::new(&mut hs, &ops, line, &hl);

        use syntect_tui::{into_span, SyntectTuiError};
        let spans: Result<Vec<Span>, SyntectTuiError> = iter.map(|t| into_span(t)).collect();
        // Remove background color
        let spans: Vec<Span> = spans?.into_iter().map(|s| {
            s.bg(ratatui::style::Color::Reset)
        }).collect();
        token_lines.push(Line::from(spans));
    }

    Ok(HighLightCache {
        lines: token_lines,
        dirty: false,
    })
}
