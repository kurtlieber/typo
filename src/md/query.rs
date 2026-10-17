//! Read-only views of a document: the outline, word counts and find /
//! replace matches.

use std::ops::Range;

use regex::{Regex, RegexBuilder};

use super::inline::{self, Kind, Node, Options, Refs};
use super::model::{BlockKind, ContainerKind, Document};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heading {
    pub block: usize,
    pub level: u8,
    pub title: String,
}

/// The document's headings with their plain titles.
pub fn outline(doc: &Document, refs: &Refs, opts: &Options) -> Vec<Heading> {
    let mut out = Vec::new();
    for (i, b) in doc.blocks.iter().enumerate() {
        if let BlockKind::Heading { level, .. } = b.kind {
            out.push(Heading { block: i, level, title: plain(&b.text, refs, opts) });
        }
    }
    out
}

/// GitHub's alert kinds, which Typora draws as callouts.
pub const ALERTS: [&str; 5] = ["note", "tip", "important", "warning", "caution"];

/// The alert a quote's first paragraph starts, with `[!NOTE]` alone on its
/// first line: the kind and the marker's length in bytes.
pub fn alert_of(text: &str) -> Option<(&'static str, usize)> {
    let mark = text.split('\n').next()?.trim_end();
    let kind = mark.strip_prefix("[!")?.strip_suffix(']')?;
    ALERTS.iter().find(|k| k.eq_ignore_ascii_case(kind)).map(|k| (*k, mark.len()))
}

/// Emoji suggestions for the shortcode being typed (`token`, without its
/// colons), as Typora orders them: shortcodes starting with it, then those
/// with a word after `_` or `-` starting with it (ignoring case), each in
/// sorted order, with an exact match first. Pairs of shortcode and emoji.
pub fn emoji_suggestions(token: &str) -> Vec<(&'static str, &'static str)> {
    if token.is_empty() {
        return Vec::new();
    }
    let mut all: Vec<(&'static str, &'static str)> =
        emojis::iter().flat_map(|e| e.shortcodes().map(move |c| (c, e.as_str()))).collect();
    all.sort_unstable();
    all.dedup_by_key(|p| p.0);
    let lower = token.to_lowercase();
    let word = |name: &str| {
        let name = name.to_lowercase();
        name.starts_with(&lower) || name.match_indices(&lower).any(|(i, _)| i > 0 && matches!(name.as_bytes()[i - 1], b'_' | b'-'))
    };
    let (mut out, rest): (Vec<_>, Vec<_>) = all.into_iter().partition(|p| p.0.starts_with(token));
    out.extend(rest.into_iter().filter(|p| p.0 != token && word(p.0)));
    if let Some(i) = out.iter().position(|p| p.0 == token) {
        let exact = out.remove(i);
        out.insert(0, exact);
    }
    // Then those whose name has it as a word, as Typora's keyword map
    // (which is its own data) adds those for the word: "heart" finds 💘.
    for e in emojis::iter() {
        let Some(code) = e.shortcode() else { continue };
        let named = e.name().split(|c: char| !c.is_alphanumeric()).any(|w| w.eq_ignore_ascii_case(token));
        if named && !out.iter().any(|p| p.1 == e.as_str()) {
            out.push((code, e.as_str()));
        }
    }
    out
}

/// A footnote: where it is referred to and where it is defined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Footnote {
    /// The label as first written.
    pub label: String,
    /// Each reference, in document order.
    pub refs: Vec<NoteRef>,
    /// The first and last block of its definition.
    pub def: Option<(usize, usize)>,
}

/// A reference to a footnote: its block, its cell (row, column) when the
/// block is a table, and its byte range (`[^label]`) in that text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteRef {
    pub block: usize,
    pub cell: Option<(usize, usize)>,
    pub range: Range<usize>,
}

impl NoteRef {
    /// The text `range` is in: the block's, or its cell's.
    pub fn text<'a>(&self, doc: &'a Document) -> &'a str {
        let b = &doc.blocks[self.block];
        match (&b.kind, self.cell) {
            (BlockKind::Table { rows, .. }, Some((r, c))) => &rows[r][c],
            _ => &b.text,
        }
    }
}

/// Labels match without regard to case, as in GFM and Typora.
pub fn footnote_key(label: &str) -> String {
    label.to_lowercase()
}

/// The document's footnotes, in order of first reference or definition.
pub fn footnotes(doc: &Document, refs: &Refs, opts: &Options) -> Vec<Footnote> {
    fn walk(nodes: &[Node], block: usize, cell: Option<(usize, usize)>, out: &mut Vec<Footnote>) {
        for n in nodes {
            if let Kind::FootnoteRef { label } = &n.kind {
                entry(out, label).refs.push(NoteRef { block, cell, range: n.range.clone() });
            }
            walk(&n.children, block, cell, out);
        }
    }
    fn entry<'a>(out: &'a mut Vec<Footnote>, label: &str) -> &'a mut Footnote {
        let key = footnote_key(label);
        let i = match out.iter().position(|f| footnote_key(&f.label) == key) {
            Some(i) => i,
            None => {
                out.push(Footnote { label: label.to_string(), refs: Vec::new(), def: None });
                out.len() - 1
            }
        };
        &mut out[i]
    }
    let mut out = Vec::new();
    for (i, b) in doc.blocks.iter().enumerate() {
        for c in &b.containers {
            if let ContainerKind::Footnote { label } = &c.kind {
                let f = entry(&mut out, label);
                // The first definition counts, as in GFM.
                match f.def {
                    None => f.def = Some((i, i)),
                    Some((first, last)) if last + 1 == i && doc.blocks[first].containers.iter().any(|d| d.id == c.id) => {
                        f.def = Some((first, i))
                    }
                    _ => {}
                }
            }
        }
        if b.kind.is_inline_text() {
            walk(&inline::parse(&b.text, refs, opts), i, None, &mut out);
        }
        if let BlockKind::Table { rows, .. } = &b.kind {
            for (r, row) in rows.iter().enumerate() {
                for (c, text) in row.iter().enumerate() {
                    walk(&inline::parse(text, refs, opts), i, Some((r, c)), &mut out);
                }
            }
        }
    }
    out
}

/// Footnote `label`'s definition as a document of its own, for a preview,
/// with the document's link definitions; `None` when it has none.
pub fn footnote_doc(doc: &Document, refs: &Refs, opts: &Options, label: &str) -> Option<Document> {
    let key = footnote_key(label);
    let (first, last) = footnotes(doc, refs, opts).into_iter().find(|f| footnote_key(&f.label) == key)?.def?;
    let mut blocks: Vec<_> = doc.blocks[first..=last]
        .iter()
        .map(|b| {
            let mut b = b.clone();
            if let Some(at) = b.containers.iter().position(|c| matches!(&c.kind, ContainerKind::Footnote { label } if footnote_key(label) == key)) {
                b.containers.drain(..=at);
            }
            b
        })
        .collect();
    blocks.extend(doc.blocks.iter().filter(|b| matches!(b.kind, BlockKind::LinkRefDef)).cloned());
    Some(Document { blocks, ..Document::default() })
}

/// A heading's anchor for `[text](#anchor)` links, GitHub style: lower
/// case, punctuation dropped, spaces as hyphens.
pub fn slug(title: &str) -> String {
    title
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// The row of the heading with anchor `anchor`; repeated titles get `-1`,
/// `-2`… after the first.
pub fn heading_with_anchor(doc: &Document, refs: &Refs, opts: &Options, anchor: &str) -> Option<usize> {
    let want = anchor.to_lowercase();
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for h in outline(doc, refs, opts) {
        let base = slug(&h.title);
        let n = seen.entry(base.clone()).or_insert(0);
        let slug = if *n == 0 { base } else { format!("{base}-{n}") };
        *n += 1;
        if slug == want {
            return Some(h.block);
        }
    }
    None
}

fn plain(src: &str, refs: &Refs, opts: &Options) -> String {
    let mut s = String::new();
    inline::plain_text(src, &inline::parse(src, refs, opts), &mut s);
    s
}

/// The text a reader sees in each block: inline markup is rendered away,
/// code and math count as written, and rules, TOCs, front matter and link
/// definitions count for nothing.
pub fn visible_text(doc: &Document, refs: &Refs, opts: &Options) -> Vec<String> {
    doc.blocks
        .iter()
        .map(|b| match &b.kind {
            BlockKind::Paragraph | BlockKind::Heading { .. } => plain(&b.text, refs, opts),
            BlockKind::CodeFence { .. } | BlockKind::IndentedCode | BlockKind::Math { .. } | BlockKind::Html => b.text.clone(),
            BlockKind::Table { rows, .. } => {
                rows.iter().flatten().map(|c| plain(c, refs, opts)).collect::<Vec<_>>().join(" ")
            }
            _ => String::new(),
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub words: usize,
    pub characters: usize,
    pub lines: usize,
}

impl Stats {
    /// Reading time in whole minutes at `wpm` words a minute, rounded as
    /// Typora rounds it (it shows 0 as "≤ 1").
    pub fn minutes(&self, wpm: usize) -> usize {
        let wpm = wpm.max(1);
        (2 * self.words + wpm) / (2 * wpm)
    }
}

/// JavaScript's `\s`.
fn js_space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

fn line_end(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// ASCII punctuation and CJK symbols, which make no word on their own.
fn lone_punct(c: char) -> bool {
    matches!(c, '!'..='/' | ':'..='@' | '['..='`' | '{'..='~' | '\u{3000}'..='\u{303f}')
}

/// What separates words: spaces and punctuation, except `-`, `.` and `/`.
fn separator(c: char) -> bool {
    js_space(c) || matches!(c, '!'..=',' | '\\' | ':'..='@' | '['..='`' | '{'..='~' | '\u{3000}'..='\u{303f}')
}

/// Typora's word count of `text`, as Typora counts it: each CJK
/// character (and each half of a character beyond the BMP, emoji
/// included) is a word; an apostrophe joins what follows it to the word
/// before; punctuation standing alone is no word; and the rest is split on
/// spaces and punctuation other than `-`, `.` and `/` (so `well-known`,
/// `3.5` and `a/b` are one word each, and `a_b` two).
pub fn words(text: &str) -> usize {
    if text.chars().all(js_space) {
        return 0;
    }
    let mut cjk = 0;
    let mut s: Vec<char> = Vec::with_capacity(text.len());
    for c in text.chars() {
        match c as u32 {
            0x3040..=0xABFF | 0xD7A4..=0xFAFF => {
                cjk += 1;
                s.push(' ');
            }
            // Its two UTF-16 halves fall in the range above.
            0x10000.. => {
                cjk += 2;
                s.extend("  ".chars());
            }
            _ => s.push(c),
        }
    }
    // `'\w+` becomes "b".
    let mut t: Vec<char> = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if matches!(s[i], '\'' | '’') && s.get(i + 1).is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_') {
            t.push('b');
            i += 1;
            while s.get(i).is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_') {
                i += 1;
            }
        } else {
            t.push(s[i]);
            i += 1;
        }
    }
    // `(^|\s+)P+(\s+|$)` (multiline) becomes a space, backtracking as a
    // regex does, since U+3000 is both a space and punctuation.
    let at = |i: usize| -> Option<usize> {
        let mut starts = Vec::new();
        if i == 0 || line_end(t[i - 1]) {
            starts.push(i);
        }
        let mut j = i;
        while j < t.len() && js_space(t[j]) {
            j += 1;
        }
        starts.extend((i + 1..=j).rev());
        for p in starts {
            let mut k = p;
            while k < t.len() && lone_punct(t[k]) {
                k += 1;
            }
            for q in (p + 1..=k).rev() {
                if q < t.len() && js_space(t[q]) {
                    let mut r = q;
                    while r < t.len() && js_space(t[r]) {
                        r += 1;
                    }
                    return Some(r);
                }
                if q == t.len() {
                    return Some(q);
                }
            }
        }
        None
    };
    let mut u: Vec<char> = vec!['d', ' '];
    let mut i = 0;
    while i < t.len() {
        if let Some(end) = at(i) {
            u.push(' ');
            i = end;
        } else {
            u.push(t[i]);
            i += 1;
        }
    }
    u.extend([' ', 'd']);
    let runs = u.iter().zip(u.iter().skip(1)).filter(|(a, b)| !separator(**a) && separator(**b)).count();
    cjk + runs + 1 - 2
}

/// A block's inline markdown as Typora's editor holds it in the page,
/// which is what it counts: the markup is there, hidden, and an emoji is
/// its `:name:` between zero-width spaces.
fn page_text(src: &str, refs: &Refs, opts: &Options) -> String {
    fn emoji(nodes: &[Node], out: &mut Vec<Range<usize>>) {
        for n in nodes {
            if matches!(n.kind, Kind::Emoji(_)) {
                out.push(n.range.clone());
            } else {
                emoji(&n.children, out);
            }
        }
    }
    let mut marks = Vec::new();
    emoji(&inline::parse(src, refs, opts), &mut marks);
    let mut s = String::with_capacity(src.len() + 6 * marks.len());
    let mut at = 0;
    for r in marks {
        s.push_str(&src[at..r.start]);
        s.push('\u{200b}');
        s.push_str(&src[r.clone()]);
        s.push('\u{200b}');
        at = r.end;
    }
    s.push_str(&src[at..]);
    s
}

/// What Typora's word count reads, a line per block: inline markdown as
/// written (see `page_text`), each table cell, a footnote definition's
/// label and a ↩ for each reference to it, a fence's "```lang" line,
/// math's `$$` lines, "toc" for a TOC, and nothing for a rule.
pub fn counted_text(doc: &Document, refs: &Refs, opts: &Options) -> String {
    let notes = footnotes(doc, refs, opts);
    let mut out = String::new();
    for (i, b) in doc.blocks.iter().enumerate() {
        let text = match &b.kind {
            BlockKind::Paragraph | BlockKind::Heading { .. } => {
                let text = page_text(&b.text, refs, opts);
                match notes.iter().find(|f| f.def.is_some_and(|(first, _)| first == i)) {
                    Some(f) => format!("{}\u{a0}{}{}", f.label, text, " ↩".repeat(f.refs.len())),
                    None => text,
                }
            }
            BlockKind::CodeFence { info, .. } => {
                let lang = info.trim();
                if lang.is_empty() { b.text.clone() } else { format!("```{lang}\n{}```", b.text) }
            }
            BlockKind::Math { .. } => format!("$$\n{}\n$$", b.text),
            BlockKind::Table { rows, .. } => {
                rows.iter().flatten().map(|c| page_text(c, refs, opts)).collect::<Vec<_>>().join("\n")
            }
            BlockKind::Toc => "toc".into(),
            BlockKind::ThematicBreak { .. } => String::new(),
            _ => b.text.clone(),
        };
        if !text.is_empty() {
            out.push('\n');
            out.push_str(&text);
        }
    }
    out
}

/// What Typora counts in a selection, `part` (from `edit::slice`): each
/// block's text as the page holds it, a line each when there are several.
pub fn selected_text(part: &Document, refs: &Refs, opts: &Options) -> String {
    let texts: Vec<String> = part
        .blocks
        .iter()
        .map(|b| match &b.kind {
            BlockKind::Paragraph | BlockKind::Heading { .. } => page_text(&b.text, refs, opts),
            BlockKind::Table { rows, .. } => cells_text(rows.iter().flatten(), refs, opts),
            BlockKind::Toc => "[toc]".into(),
            BlockKind::ThematicBreak { .. } => String::new(),
            _ => b.text.clone(),
        })
        .collect();
    match texts.as_slice() {
        [one] => one.clone(),
        _ => texts.iter().map(|t| format!("{t}\n")).collect(),
    }
}

/// Table cells' text, a line each, as a selection over them reads.
pub fn cells_text<'a>(cells: impl IntoIterator<Item = &'a String>, refs: &Refs, opts: &Options) -> String {
    cells.into_iter().map(|c| page_text(c, refs, opts) + "\n").collect()
}

/// The status bar's counts, as Typora's: words as `words` counts them,
/// and the markdown's characters (UTF-16 units, line breaks included) and
/// lines.
pub fn stats(doc: &Document, source: &str, refs: &Refs, opts: &Options) -> Stats {
    Stats {
        words: words(&counted_text(doc, refs, opts)),
        characters: source.encode_utf16().count(),
        lines: source.matches('\n').count() + 1,
    }
}

// ---------------------------------------------------------------------------
// Find and replace, over each block's markdown text and table cells.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub block: usize,
    /// `(row, column)` for a match in a table cell.
    pub cell: Option<(usize, usize)>,
    pub range: Range<usize>,
}

pub fn matcher(query: &str, opts: &FindOptions) -> Result<Regex, regex::Error> {
    let mut pattern = if opts.regex { query.to_string() } else { regex::escape(query) };
    if opts.whole_word {
        pattern = format!(r"\b(?:{pattern})\b");
    }
    RegexBuilder::new(&pattern).case_insensitive(!opts.case_sensitive).build()
}

pub fn find(doc: &Document, query: &str, opts: &FindOptions) -> Result<Vec<Match>, regex::Error> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let re = matcher(query, opts)?;
    let mut out = Vec::new();
    for (i, b) in doc.blocks.iter().enumerate() {
        match &b.kind {
            BlockKind::Table { rows, .. } => {
                for (r, row) in rows.iter().enumerate() {
                    for (c, cell) in row.iter().enumerate() {
                        for m in re.find_iter(cell).filter(|m| !m.is_empty()) {
                            out.push(Match { block: i, cell: Some((r, c)), range: m.range() });
                        }
                    }
                }
            }
            // Neither shows its text: a rule, and the TOC's `[toc]`.
            BlockKind::ThematicBreak { .. } | BlockKind::Toc => {}
            _ => {
                for m in re.find_iter(&b.text).filter(|m| !m.is_empty()) {
                    out.push(Match { block: i, cell: None, range: m.range() });
                }
            }
        }
    }
    Ok(out)
}

/// The replacement text for one match (`$1` refers to groups in regex mode).
pub fn replacement(doc: &Document, m: &Match, query: &str, with: &str, opts: &FindOptions) -> Result<String, regex::Error> {
    let re = matcher(query, opts)?;
    let text = match (&doc.blocks[m.block].kind, m.cell) {
        (BlockKind::Table { rows, .. }, Some((r, c))) => rows[r][c].as_str(),
        _ => doc.blocks[m.block].text.as_str(),
    };
    if !opts.regex {
        return Ok(with.to_string());
    }
    Ok(expand(&re, text, m.range.clone(), with).unwrap_or_default())
}

/// `with` for the match at `range` of `text`, its `$1` filled in, if it
/// still matches there. Matching again in place lets anchors and
/// lookaround see the whole text.
fn expand(re: &Regex, text: &str, range: Range<usize>, with: &str) -> Option<String> {
    let caps = re.captures_at(text, range.start).filter(|c| c.get(0).is_some_and(|g| g.range() == range))?;
    let mut out = String::new();
    caps.expand(with, &mut out);
    Some(out)
}

/// Matches in plain text: Source Code Mode's markdown.
pub fn find_in(text: &str, query: &str, opts: &FindOptions) -> Result<Vec<Range<usize>>, regex::Error> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let re = matcher(query, opts)?;
    Ok(re.find_iter(text).filter(|m| !m.is_empty()).map(|m| m.range()).collect())
}

/// What replaces the match at `range` of plain `text`, or `None` when it no
/// longer matches there.
pub fn replacement_in(text: &str, range: Range<usize>, query: &str, with: &str, opts: &FindOptions) -> Option<String> {
    let re = matcher(query, opts).ok()?;
    let found = expand(&re, text, range, if opts.regex { with } else { "" })?;
    Some(if opts.regex { found } else { with.to_string() })
}

/// Replace one match.
pub fn replace(doc: &mut Document, m: &Match, query: &str, with: &str, opts: &FindOptions) -> Result<(), regex::Error> {
    let new = replacement(doc, m, query, with, opts)?;
    let b = &mut doc.blocks[m.block];
    if let (BlockKind::Table { align, rows }, Some((r, c))) = (&b.kind, m.cell) {
        let (align, mut rows) = (align.clone(), rows.clone());
        rows[r][c].replace_range(m.range.clone(), &new);
        b.set_kind(BlockKind::Table { align, rows });
    } else {
        let mut text = b.text.clone();
        text.replace_range(m.range.clone(), &new);
        b.set_text(text);
    }
    Ok(())
}

/// Replace every match; returns how many there were.
pub fn replace_all(doc: &mut Document, query: &str, with: &str, opts: &FindOptions) -> Result<usize, regex::Error> {
    let matches = find(doc, query, opts)?;
    // Back to front, so earlier ranges stay valid.
    for m in matches.iter().rev() {
        replace(doc, m, query, with, opts)?;
    }
    Ok(matches.len())
}
