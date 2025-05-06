//! For all your parsing and highlighting needs

use std::borrow::Cow;
use std::collections::HashMap;

use ratatui::style::Stylize;
use ratatui::text::{Span, Line};
use syntect::parsing::{ParseState, SyntaxReference, ScopeStack, SyntaxSet};
use syntect::highlighting::{HighlightState, Highlighter, HighlightIterator};
use syntect::util::LinesWithEndings;
use tracing::debug;
use unicode_segmentation::UnicodeSegmentation;
use crate::syntect_tui::{self, SyntectTuiError};

const CACHE_FREQUENCY: usize = 10;

pub mod whitespace {
    pub const TABSIZE: usize = 4;
    // https://www.emacswiki.org/emacs/ShowWhiteSpace
    pub const LF: &'static str = "¶\n"; // pilcrow
    pub const CR: &'static str = "⁋";
    //static LF: &'static str = "\n$";
    pub const SPACE: &'static str = "·";
}

pub trait ParseCacheTrait {
    fn invalidate_from(&mut self, from: usize);
    fn closest_state(&self, from: usize) -> Option<(usize, &CachedParseState)> ;
}

pub type ParseCache = HashMap<usize, CachedParseState>;

impl ParseCacheTrait for ParseCache {
    fn invalidate_from(&mut self, from: usize) {
        self.retain(|&k, _| k < from);
    }
    /// Find the closest usable cache state for a specific line
    fn closest_state(&self, from: usize) -> Option<(usize, &CachedParseState)> {
        for i in (0..from).rev() {
            if let Some(state) = self.get(&i) {
                return Some((i, state));
            }
        }
        return None;
    }
}

/// Replace characters
/// in the future this should be able to take non-utf8 and create utf8 strings
/// for binary editing
/// it is important that whitespace replacements don't change the length for now, because
/// the whitespace setting isn't globally known
pub fn perform_str_replacements<'a>(str: &'a str, decorate_whitespace: bool) -> Cow<'a, str> {
    let cow: Cow<'a, str> = Cow::Borrowed(&str);
    let toreplace = if decorate_whitespace {
        vec!['\t', '\n', '\r', ' ']
    } else {
        vec!['\t']
    };
    if str.chars().any(|c| toreplace.contains(&c)) {
        if decorate_whitespace {
            cow
            .replace("\t", &"↦".repeat(whitespace::TABSIZE))
            .replace("\n", whitespace::LF)
            .replace("\r", whitespace::CR)
            .replace(" ", whitespace::SPACE).into()
        } else {
            cow
            .replace("\t", &" ".repeat(whitespace::TABSIZE)).into()
        }
    } else {
        cow
    }
}

#[tracing::instrument(skip_all, level="trace", fields(start, limit = limit, from = from, n))]
pub fn parse_from<'a>(from: usize, lines: LinesWithEndings<'a>, limit: usize, cache: &mut HashMap<usize, CachedParseState>, highlighter: &Highlighter, syntax: &SyntaxReference, syntax_set: &SyntaxSet, show_whitespace: bool) 
-> anyhow::Result<Vec<Line<'a>>> {
    let (start, mut state) = match cache.closest_state(from) {
        Some((i, state)) => (i, state.clone()),
        None => (0, CachedParseState::new(highlighter, syntax)),
    };

    tracing::Span::current().record("start", start).record("n", from + limit - start);

    let mut lexemes: Vec<Line<'a>> = vec![];

    for (line_no, line) in lines.enumerate() {
        if line_no < start {
            continue;
        }
        // Possibly cache the state
        if line_no % CACHE_FREQUENCY == 0 {
            cache.insert(line_no, state.clone());
        }

        let ops = state.ps.parse_line(line, syntax_set)?;
        let iter = HighlightIterator::new(&mut state.hs, &ops, line, highlighter);
        
        let spans: Result<Vec<Span>, SyntectTuiError> = iter.map(|t| syntect_tui::into_span(t)).collect();
        
        // I need some kind of global preprocessor here
        // it will move whitespace to seperate spans (also color them)
        // then it will replace parts of spans (tabs with 4 spaces, whitespace with symbols)
        // those replacents should be registered somewhere, so other functions can replicate
        // the line length difference
        // the functions that use that are str_column_length and crate::wrap::get_linebreak_locations

        if line_no >= from {
            // Remove background color and handle whitespace chars
            let spans: Vec<Span> = spans?.into_iter().map(|mut s| {
                // not all parsers create separate spans for the whitespace
                // I have to figure out a method to break up spans
                // otherwise I cannot color the whitespace appropiately
                let content = perform_str_replacements(&s.content, show_whitespace).into_owned();
                s = s.content(content);
                if s.style.bg.is_none() {
                    s = s.fg(ratatui::style::Color::Reset);
                }
                s.bg(ratatui::style::Color::Reset)
            }).collect();

            let breaks = crate::wrap::get_linebreak_locations(&line, 10000);
            // this is the glorious linebreak span insertion apparatus
            // given a list of spans and a list of linebreaks
            // it will generate broken lines
            if breaks.len() > 0 {
                let mut new_spans = vec![];
                let mut break_i = 0;
                let mut row = 0;
                'outer: for i in 0..spans.len() {
                    let span = &spans[i];
                    let span_len = spans[i].content.graphemes(true).count();
                    // check if no break occurs in this span
                    if row + span_len < breaks[break_i] {
                        new_spans.push(spans[i].clone());
                    } else {
                        let mut span_deepness = 0;
                        // loop through span to split it up
                        loop {
                            let style = spans[i].style;
                            debug!("deepenss {} break {}", span_deepness, break_i);
                            new_spans.push(Span::styled(span.content[span_deepness..breaks[break_i]].to_owned(), style));
                            lexemes.push(Line::from(new_spans));
                            new_spans = vec![];
                            span_deepness = breaks[break_i] - row;
                            break_i += 1;
                            if break_i >= breaks.len() {
                                debug!("deepenss {} end", span_deepness);
                                new_spans.push(Span::styled(span.content[span_deepness..].to_owned(), style));
                                lexemes.push(Line::from(new_spans.clone()));
                                break 'outer;
                            }
                        }
                    }
                    row += span_len;
                    lexemes.push(Line::from(new_spans.clone()));
                }
            } else {
                lexemes.push(Line::from(spans));
            }
        }

        if line_no > from+limit {
            break;
        }
    }

    return Ok(lexemes);
}

// Parse
#[derive(Clone, Debug)]
pub struct CachedParseState {
    pub ps: ParseState,
    pub hs: HighlightState,
}

impl CachedParseState {
    pub fn new(highlighter: &Highlighter, syntax: &SyntaxReference) -> CachedParseState {
        CachedParseState {
            ps: ParseState::new(syntax),
            hs: HighlightState::new(highlighter, ScopeStack::new()),
        }
    }
}
