//! `SourceHighlighter`: Source Code Mode's colours. Each line of the
//! markdown is a block of the view's `TextEdit` document, read by
//! `source::line` with the state the line before it left, which Qt keeps
//! per block and re-highlights onwards from when it changes. Its bridge
//! lives with the inline highlighter's, which shares its C++ base.

use core::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QRectF, QString};

use super::editor::{byte_of, utf16_of};
use super::highlighter::{BOLD, Dim, ITALIC, MONO, STRIKE, UNDERLINE, hit_ranges, qobject};
use super::shared;
use crate::code::{States, Token};
use crate::source::{self, Block, Pair, State, Typing};

pub struct SourceHighlighterRust {
    pub(crate) font_size: f64,
    pub(crate) hits: QString,
    pub(crate) doc: i32,
    dim: Dim,
    /// Fences' parser states, which lines' states name.
    codes: States,
}

impl Default for SourceHighlighterRust {
    fn default() -> Self {
        SourceHighlighterRust { font_size: 16.0, hits: QString::from("[]"), doc: -1, dim: Dim::default(), codes: States::default() }
    }
}

impl qobject::SourceHighlighter {
    pub(crate) unsafe fn attach(self: Pin<&mut Self>, doc: *mut qobject::QQuickTextDocument) {
        unsafe { self.attach_quick_document(doc) };
    }

    pub(crate) fn refresh(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().codes.clear();
        self.rehighlight_all();
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

    pub(crate) fn fix_line_height(mut self: Pin<&mut Self>) {
        let px = self.font_size * 1.6;
        self.as_mut().apply_line_height(px, true, 0.0);
        self.clear_undo();
    }

    pub(crate) fn replace_text(self: Pin<&mut Self>, start: i32, end: i32, text: &QString, join: bool) {
        self.replace_range_base(start, end, text, join);
    }

    pub(crate) fn line_number_at(&self, pos: i32) -> i32 {
        self.doc_line_number(pos)
    }

    pub(crate) fn line_start_at(&self, pos: i32) -> i32 {
        self.doc_line_start(pos)
    }

    pub(crate) fn line_position(&self, line: i32) -> i32 {
        self.doc_line_position(line)
    }

    pub(crate) fn line_rect(&self, pos: i32) -> QRectF {
        self.doc_line_rect(pos)
    }

    pub(crate) fn ink_at(&self, pos: i32) -> QString {
        self.doc_ink_at(pos)
    }

    pub(crate) fn restyle_above(self: Pin<&mut Self>, pos: i32) {
        let n = self.doc_line_number(pos);
        if n > 0 {
            self.rehighlight_line(n - 1);
        }
    }

    pub(crate) fn enter_at(&self, start: i32, end: i32, same: bool, pairs: bool, tab: i32) -> QString {
        let (first, last) = (self.doc_line_text(start).to_string(), self.doc_line_text(end).to_string());
        let (head, tail) = (self.doc_line_start(start), self.doc_line_start(end));
        let before = &first[..byte_of(&first, start - head)];
        let after = &last[byte_of(&last, end - tail)..];
        let prev = State::decode(self.doc_state_before(start));
        let t = source::enter(before, after, start != end, prev, Typing { same, pairs, tab: tab.max(1) as usize });
        let from = head + utf16_of(before, t.keep);
        let caret = from + utf16_of(&t.text, t.caret);
        let to = end + utf16_of(after, t.skip);
        QString::from(&serde_json::json!([from, to, t.text, caret, caret]).to_string())
    }

    pub(crate) fn indent_at(&self, anchor: i32, caret: i32, add: bool, tab: i32) -> QString {
        let text = |n: i32| self.doc_line_text(self.doc_line_position(n)).to_string();
        indent_lines(|p| self.doc_line_number(p), |n| self.doc_line_position(n), text, anchor, caret, add, tab)
    }

    pub(crate) fn pair_for(&self, ch: &QString, prev: &QString, next: &QString, selected: bool) -> QString {
        // The line's ends have nothing beyond them.
        let one = |s: &QString| s.to_string().chars().next().filter(|c| !matches!(c, '\n' | '\u{2028}' | '\u{2029}'));
        let Some(c) = one(ch) else { return QString::from("[\"type\"]") };
        let reply = match source::pair(c, one(prev), one(next), selected) {
            Pair::Type => serde_json::json!(["type"]),
            Pair::Skip => serde_json::json!(["skip"]),
            Pair::Wrap(l, r) => serde_json::json!(["wrap", l.to_string(), r.to_string()]),
            Pair::Both(l, r) => serde_json::json!(["both", l.to_string(), r.to_string()]),
        };
        QString::from(&reply.to_string())
    }

    pub(crate) fn pair_around(&self, prev: &QString, next: &QString) -> bool {
        let one = |s: &QString| s.to_string().chars().next();
        matches!((one(prev), one(next)), (Some(p), Some(n)) if source::pair_around(p, n))
    }

    pub(crate) fn highlight_text(mut self: Pin<&mut Self>, text: &QString) {
        let Some(ink) = shared::ink() else { return };
        let line = text.to_string();
        let prev = State::decode(self.previous_state());
        let first = self.block_number() == 0;
        let next = self.next_line_text().to_string();
        let (layers, mut state) = shared::with_inline(self.doc, |refs, opts| source::line(&line, prev, first, &next, refs, opts));
        let px = self.font_size;
        for (r, look) in source::runs(line.len(), &layers) {
            let (s, e) = (utf16_of(&line, r.start), utf16_of(&line, r.end));
            let flags = [(look.bold, BOLD), (look.italic, ITALIC), (look.underline, UNDERLINE), (look.strike, STRIKE), (look.mono, MONO)]
                .iter()
                .filter(|(on, _)| *on)
                .fold(0, |f, (_, bit)| f | bit);
            let size = if look.scale() != 1.0 { px * look.scale() } else { 0.0 };
            let fg = look.ink.map_or(0, |i| ink.source[i as usize]);
            let bg = if look.mark { ink.mark } else { 0 };
            self.as_mut().apply_format(s, e - s, flags, size, fg, bg);
        }
        // A fence's code in its language's colours, as in code blocks.
        let fence = |s: State| matches!(s.block, Block::Fence { .. });
        if fence(prev) && fence(state) {
            let at = source::fence_body(&line);
            let body = &line[at..];
            if let Some((runs, next)) = self.as_mut().rust_mut().codes.line(prev.code, body) {
                for (r, token) in runs {
                    let (s, e) = (utf16_of(&line, at + r.start), utf16_of(&line, at + r.end));
                    let i = Token::ALL.iter().position(|t| *t == token).unwrap_or(0);
                    let flags = MONO | if token.style().1 { ITALIC } else { 0 };
                    self.as_mut().apply_format(s, e - s, flags, 0.0, ink.tokens[i], 0);
                }
                state.code = next;
            }
        } else if fence(state) {
            state.code = source::fence_info(&line).map_or(0, |info| self.as_mut().rust_mut().codes.start(&info));
        }
        // Find's matches on this line.
        let hits = hit_ranges(&self.hits);
        if !hits.is_empty() {
            let base = self.block_position();
            let end = base + utf16_of(&line, usize::MAX);
            for h in hits {
                let (s, e) = (h.start.max(base), h.end.min(end));
                if s < e {
                    self.as_mut().add_background(s - base, e - s, ink.find);
                }
            }
        }
        if self.dim.greys(self.block_number()) {
            self.as_mut().dim_line(ink.blur);
        }
        self.as_mut().set_block_state(state.encode());
    }
}

/// `indentAt` over a document read by `line_of` (a position's line),
/// `start_of` (a line's position) and `text_of` (a line's text), Source
/// Code Mode's or a code fence's.
pub(crate) fn indent_lines(
    line_of: impl Fn(i32) -> i32,
    start_of: impl Fn(i32) -> i32,
    text_of: impl Fn(i32) -> String,
    anchor: i32,
    caret: i32,
    add: bool,
    tab: i32,
) -> QString {
    let (start, end) = (anchor.min(caret), anchor.max(caret));
    let selected = start != end;
    let first = line_of(start);
    let mut last = line_of(end);
    // A selection ending at a line's start leaves that line be.
    if selected && last > first && end == start_of(last) {
        last -= 1;
    }
    let lines: Vec<String> = (first..=last).map(&text_of).collect();
    let above = (first > 0).then(|| text_of(first - 1));
    let from = start_of(first);
    let to = start_of(last) + utf16_of(&lines[lines.len() - 1], usize::MAX);
    // Where the ends sit, as (line, byte column) in `lines`.
    let mark = |pos: i32| {
        let n = (line_of(pos) - first) as usize;
        let line = &lines[n.min(lines.len() - 1)];
        (n, byte_of(line, pos - start_of(line_of(pos))))
    };
    let inside = |pos: i32| pos <= to;
    let marks: Vec<(usize, usize)> = [anchor, caret].into_iter().filter(|&p| inside(p)).map(mark).collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let (new, cols) = source::reindent(&refs, above.as_deref(), add, selected, tab.max(1) as usize, &marks);
    let joined = new.join("\n");
    let grown = utf16_of(&joined, usize::MAX) - (to - from);
    // A mark's position in the new text.
    let place = |(n, col): (usize, usize)| {
        let skipped: i32 = new[..n].iter().map(|l| utf16_of(l, usize::MAX) + 1).sum();
        from + skipped + utf16_of(&new[n], col)
    };
    let mut cols = cols.into_iter();
    let mut moved = |pos: i32| {
        if !inside(pos) {
            return pos + grown;
        }
        let (n, col) = mark(pos);
        let at = place((n, cols.next().unwrap_or(col)));
        // A selection from a line's start keeps starting there.
        if selected && pos == start && col == 0 { from + new[..n].iter().map(|l| utf16_of(l, usize::MAX) + 1).sum::<i32>() } else { at }
    };
    let (a, c) = (moved(anchor), moved(caret));
    QString::from(&serde_json::json!([from, to, joined, a, c]).to_string())
}
