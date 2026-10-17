//! `CodeHighlighter`: syntax colours for one code fence. Each line of the
//! fence is a block of its `TextEdit`'s document; the whole fence is parsed
//! with `code::Highlight`, which re-parses from the edited line only, and
//! each line is painted from that. Its bridge lives with the inline
//! highlighter's, which shares its C++ base.

use core::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

use super::editor::utf16_of;
use super::highlighter::{Dim, ITALIC, hit_ranges, qobject};
use super::shared;
use crate::code::{Highlight, Token};

#[derive(Default)]
pub struct CodeHighlighterRust {
    pub(crate) language: QString,
    pub(crate) hits: QString,
    pub(crate) errors: QString,
    /// The text and language last parsed.
    text: String,
    parsed_language: String,
    highlight: Highlight,
    dim: Dim,
}

impl qobject::CodeHighlighter {
    pub(crate) unsafe fn attach(self: Pin<&mut Self>, doc: *mut qobject::QQuickTextDocument) {
        unsafe { self.attach_quick_document(doc) };
    }

    pub(crate) fn refresh(mut self: Pin<&mut Self>) {
        // Forget the parse so the next paint starts over.
        self.as_mut().rust_mut().text.clear();
        self.as_mut().rust_mut().highlight = Highlight::default();
        self.rehighlight_all();
    }

    pub(crate) fn fix_line_height(self: Pin<&mut Self>, pixels: f64) {
        self.apply_line_height(pixels, false, 0.0);
    }

    pub(crate) fn dim(mut self: Pin<&mut Self>, blur: bool, keep: i32) {
        let next = Dim::new(blur, keep);
        let old = self.dim;
        if old == next {
            return;
        }
        self.as_mut().rust_mut().dim = next;
        match old.changed(next) {
            Some(lines) => lines.into_iter().for_each(|l| self.as_mut().rehighlight_line(l)),
            None => self.rehighlight_all(),
        }
    }

    pub(crate) fn highlight_text(mut self: Pin<&mut Self>, line: &QString) {
        let full = self.document_text().to_string();
        let language = self.language.to_string();
        if self.text != full || self.parsed_language != language || full.is_empty() {
            let mut rust = self.as_mut().rust_mut();
            rust.highlight.update(&language, &full);
            rust.text = full;
            rust.parsed_language = language;
        }
        let Some(ink) = shared::ink() else { return };
        let n = self.block_number().max(0) as usize;
        let line = line.to_string();
        let runs: Vec<_> = self.highlight.runs(n).to_vec();
        for (range, token) in runs {
            let (s, e) = (utf16_of(&line, range.start), utf16_of(&line, range.end));
            let i = Token::ALL.iter().position(|t| *t == token).unwrap_or(0);
            let flags = if token.style().1 { ITALIC } else { 0 };
            self.as_mut().apply_format(s, e - s, flags, 0.0, ink.tokens[i], 0);
        }
        // Find's matches and a diagram's error on this line: the lines
        // before it and their breaks come first.
        let (hits, errors) = (hit_ranges(&self.hits), hit_ranges(&self.errors));
        if !hits.is_empty() || !errors.is_empty() {
            let base: i32 = self.text.split('\n').take(n).map(|l| utf16_of(l, usize::MAX) + 1).sum();
            let end = base + utf16_of(&line, usize::MAX);
            let on_line = |h: &std::ops::Range<i32>| (h.start.max(base) - base, h.end.min(end) - base);
            for (s, e) in hits.iter().map(on_line).filter(|(s, e)| s < e) {
                self.as_mut().add_background(s, e - s, ink.find);
            }
            for (s, e) in errors.iter().map(on_line).filter(|(s, e)| s < e) {
                self.as_mut().add_wave(s, e - s, ink.error);
            }
        }
        if self.dim.greys(n as i32) {
            self.as_mut().dim_line(ink.blur);
        }
        let state = self.highlight.state_hash(n);
        self.set_block_state(state);
    }
}
