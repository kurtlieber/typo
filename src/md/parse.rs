//! Markdown → `Document`. A line-based, CommonMark-style block parser: each
//! line first matches the open containers (quotes, list items, footnote
//! definitions), may open new ones, and then feeds the current leaf. Leaves
//! are emitted with their container path, their text stripped of container
//! prefixes, and their raw source lines so untouched blocks round-trip.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::model::*;

pub fn parse(src: &str) -> Document {
    parse_with(src, true, false)
}

/// `parse`, with `latex` saying whether a `\[` line opens a math block, as
/// Typora's "LaTeX Math Delimiter" preference does, and `remap` whether
/// full-width punctuation reads as ASCII (`》 ` a quote, `- 【 】 ` a task),
/// as "Remap Unicode Punctuation on Parse" does.
pub fn parse_with(src: &str, latex: bool, remap: bool) -> Document {
    if src.is_empty() {
        return Document { final_newline: false, ..Document::default() };
    }
    let line_ending = if src.contains("\r\n") { LineEnding::CrLf } else { LineEnding::Lf };
    let final_newline = src.ends_with('\n');
    let remapped: Vec<std::borrow::Cow<str>> =
        src.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).map(|l| if remap { remap_line(l) } else { l.into() }).collect();
    let mut lines: Vec<&str> = remapped.iter().map(|l| l.as_ref()).collect();
    if final_newline {
        lines.pop();
    }

    let mut parser = Parser { latex, ..Parser::default() };
    let mut start = 0;
    if let Some(end) = front_matter_end(&lines) {
        let mut block = Block::new(BlockKind::FrontMatter, lines[1..end].join("\n"), Vec::new());
        block.blank_before = Some(Vec::new());
        block.raw = Some(lines[..=end].iter().map(|s| s.to_string()).collect());
        parser.blocks.push(block);
        start = end + 1;
    }
    for line in &lines[start..] {
        parser.line(line);
    }
    parser.close_containers_from(0);

    let mut blocks = parser.blocks;
    let mut blank_after = parser.blanks;
    mark_loose_lists(&mut blocks);
    if blocks.is_empty() {
        // A document of blank lines: one empty paragraph stands for the first.
        let mut block = Block::paragraph("");
        block.blank_before = Some(vec![0; blank_after.len().saturating_sub(1)]);
        blocks.push(block);
        blank_after.clear();
    }
    Document { blocks, line_ending, final_newline, blank_after }
}

/// `---` on the first line opens YAML front matter when a closing `---` or
/// `...` line follows.
fn front_matter_end(lines: &[&str]) -> Option<usize> {
    if lines.first()?.trim_end() != "---" {
        return None;
    }
    lines.iter().enumerate().skip(1).find(|(_, l)| matches!(l.trim_end(), "---" | "...")).map(|(i, _)| i)
}

// ---------------------------------------------------------------------------
// Line cursor with tab-stop aware indentation.

struct Cur<'a> {
    s: &'a str,
    pos: usize,
    col: usize,
    /// Columns still owed by a tab that was only partly consumed.
    partial: usize,
}

impl<'a> Cur<'a> {
    fn new(s: &'a str) -> Self {
        Cur { s, pos: 0, col: 0, partial: 0 }
    }

    /// Columns of whitespace from the cursor.
    fn indent(&self) -> usize {
        let mut cols = self.partial;
        let mut col = self.col + self.partial;
        for c in self.s[self.pos..].chars() {
            match c {
                ' ' => {
                    cols += 1;
                    col += 1;
                }
                '\t' => {
                    let w = 4 - col % 4;
                    cols += w;
                    col += w;
                }
                _ => break,
            }
        }
        cols
    }

    /// The first character after the indentation.
    fn peek_nonspace(&self) -> Option<char> {
        self.s[self.pos..].chars().find(|c| *c != ' ' && *c != '\t')
    }

    /// Consume `n` columns of whitespace, splitting a tab if needed.
    fn advance_cols(&mut self, mut n: usize) {
        while n > 0 {
            if self.partial > 0 {
                let take = self.partial.min(n);
                self.partial -= take;
                self.col += take;
                n -= take;
                continue;
            }
            match self.s[self.pos..].chars().next() {
                Some(' ') => {
                    self.pos += 1;
                    self.col += 1;
                    n -= 1;
                }
                Some('\t') => {
                    let w = 4 - self.col % 4;
                    self.pos += 1;
                    if w <= n {
                        self.col += w;
                        n -= w;
                    } else {
                        self.col += n;
                        self.partial = w - n;
                        n = 0;
                    }
                }
                _ => break,
            }
        }
    }

    fn skip_indent(&mut self) {
        let n = self.indent();
        self.advance_cols(n);
    }

    /// Consume one character (not whitespace).
    fn bump(&mut self) {
        self.partial = 0;
        if let Some(c) = self.s[self.pos..].chars().next() {
            self.pos += c.len_utf8();
            self.col += 1;
        }
    }

    /// The rest of the line, with any partly consumed tab expanded.
    fn rest(&self) -> String {
        let mut s = " ".repeat(self.partial);
        s.push_str(&self.s[self.pos..]);
        s
    }

    fn rest_is_blank(&self) -> bool {
        self.s[self.pos..].trim().is_empty()
    }

    /// The rest of the line after its indentation.
    fn content(&self) -> &'a str {
        self.s[self.pos..].trim_start_matches([' ', '\t'])
    }
}

// ---------------------------------------------------------------------------
// Block recognisers.

static ATX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(#{1,6})(?:[ \t]+(.*?))?[ \t]*$").unwrap());
static FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(`{3,}|~{3,})(.*)$").unwrap());
static ORDERED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,9})([.)])").unwrap());
static FOOTNOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[\^([^\]\s]+)\]:").unwrap());
static HTML_OPEN_CLOSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(?:<[A-Za-z][A-Za-z0-9-]*(?:\s+[A-Za-z_:][\w.:-]*(?:\s*=\s*(?:[^\s"'=<>`]+|'[^']*'|"[^"]*"))?)*\s*/?>|</[A-Za-z][A-Za-z0-9-]*\s*>)\s*$"#).unwrap()
});

const HTML_BLOCK_TAGS: &[&str] = &[
    "address", "article", "aside", "base", "basefont", "blockquote", "body", "caption", "center", "col",
    "colgroup", "dd", "details", "dialog", "dir", "div", "dl", "dt", "fieldset", "figcaption", "figure",
    "footer", "form", "frame", "frameset", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header", "hr", "html",
    "iframe", "legend", "li", "link", "main", "menu", "menuitem", "nav", "noframes", "ol", "optgroup",
    "option", "p", "param", "search", "section", "summary", "table", "tbody", "td", "tfoot", "th", "thead",
    "title", "tr", "track", "ul",
];

pub(crate) fn is_thematic_break(s: &str) -> bool {
    let s = s.trim_end();
    let Some(c) = s.chars().next() else { return false };
    if !matches!(c, '-' | '*' | '_') {
        return false;
    }
    let mut count = 0;
    for ch in s.chars() {
        if ch == c {
            count += 1;
        } else if ch != ' ' && ch != '\t' {
            return false;
        }
    }
    count >= 3
}

/// `(level, text)` for an ATX heading line.
pub(crate) fn atx_heading(s: &str) -> Option<(u8, String)> {
    let caps = ATX.captures(s)?;
    let level = caps[1].len() as u8;
    let mut text = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
    // Drop an optional closing sequence of #s preceded by a space.
    let trimmed = text.trim_end_matches('#');
    if trimmed.len() != text.len() && (trimmed.is_empty() || trimmed.ends_with([' ', '\t'])) {
        text = trimmed.trim_end().to_string();
    }
    Some((level, text))
}

/// `(fence, info)` for an opening code fence.
pub(crate) fn fence_start(s: &str) -> Option<(String, String)> {
    let caps = FENCE.captures(s)?;
    let fence = caps[1].to_string();
    let info = caps[2].trim().to_string();
    if fence.starts_with('`') && info.contains('`') {
        return None;
    }
    Some((fence, info))
}

/// Whether the line opens a math block: `Some(true)` for LaTeX's `\[`,
/// when `latex` delimiters are on, `Some(false)` for `$$`.
pub(crate) fn math_start(s: &str, latex: bool) -> Option<bool> {
    match s.trim() {
        "$$" => Some(false),
        "\\[" if latex => Some(true),
        _ => None,
    }
}

pub(crate) fn math_end(s: &str, brackets: bool) -> bool {
    s.trim() == if brackets { "\\]" } else { "$$" }
}

pub(crate) fn is_fence_close(s: &str, fence: &str) -> bool {
    let c = fence.chars().next().unwrap();
    let t = s.trim_end();
    t.len() >= fence.len() && t.chars().all(|ch| ch == c)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HtmlEnd {
    Contains(&'static str),
    Blank,
}

/// The CommonMark HTML block start conditions; `in_paragraph` rules out the
/// seventh kind, which cannot interrupt a paragraph.
pub(crate) fn html_start(s: &str, in_paragraph: bool) -> Option<HtmlEnd> {
    if !s.starts_with('<') {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    for tag in ["script", "pre", "style", "textarea"] {
        if let Some(after) = lower.strip_prefix(&format!("<{tag}")) {
            if after.is_empty() || after.starts_with([' ', '\t', '>']) {
                return Some(HtmlEnd::Contains(match tag {
                    "script" => "</script>",
                    "pre" => "</pre>",
                    "style" => "</style>",
                    _ => "</textarea>",
                }));
            }
        }
    }
    if s.starts_with("<!--") {
        return Some(HtmlEnd::Contains("-->"));
    }
    if s.starts_with("<?") {
        return Some(HtmlEnd::Contains("?>"));
    }
    if s.starts_with("<![CDATA[") {
        return Some(HtmlEnd::Contains("]]>"));
    }
    if s.len() > 2 && s.starts_with("<!") && s.as_bytes()[2].is_ascii_alphabetic() {
        return Some(HtmlEnd::Contains(">"));
    }
    let name_start = if lower.starts_with("</") { 2 } else { 1 };
    let name: String = lower[name_start..].chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
    if HTML_BLOCK_TAGS.contains(&name.as_str()) {
        let after = &lower[name_start + name.len()..];
        if after.is_empty() || after.starts_with([' ', '\t', '>']) || after.starts_with("/>") {
            return Some(HtmlEnd::Blank);
        }
    }
    // An `<img>` alone on a line stays a paragraph: Typora draws it as an
    // image, which Zoom Image writes on its own line.
    if !in_paragraph && name != "img" && HTML_OPEN_CLOSE.is_match(s) {
        return Some(HtmlEnd::Blank);
    }
    None
}

/// Whether an HTML block's text ends with its own closing marker (kinds 1–5)
/// rather than needing a blank line after it.
pub(crate) fn html_self_terminated(text: &str) -> bool {
    let first = text.lines().next().unwrap_or("");
    match html_start(first.trim_start(), false) {
        Some(HtmlEnd::Contains(end)) => text.to_ascii_lowercase().contains(end),
        _ => false,
    }
}

struct ListMarkerMatch {
    marker: ListMarker,
    width: usize,
    /// Whether nothing follows the marker on this line.
    empty: bool,
}

fn list_marker(s: &str) -> Option<ListMarkerMatch> {
    let (marker, width) = if let Some(c) = s.chars().next().filter(|c| matches!(c, '-' | '*' | '+')) {
        (ListMarker::Bullet(c), 1)
    } else {
        let caps = ORDERED.captures(s)?;
        let start: u64 = caps[1].parse().ok()?;
        let delim = caps[2].chars().next().unwrap();
        (ListMarker::Ordered { start, delim, same_numbers: false }, caps[0].len())
    };
    let after = &s[width..];
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    Some(ListMarkerMatch { marker, width, empty: after.trim().is_empty() })
}

/// Column alignment for each cell of a GFM delimiter row.
pub(crate) fn delimiter_row(s: &str) -> Option<Vec<Align>> {
    if !s.contains('|') && !s.contains(':') {
        return None;
    }
    let cells = split_row(s);
    if cells.is_empty() {
        return None;
    }
    cells
        .iter()
        .map(|c| {
            let c = c.trim();
            let left = c.starts_with(':');
            let right = c.ends_with(':');
            let dashes = c.trim_matches(':');
            if dashes.is_empty() || !dashes.chars().all(|ch| ch == '-') {
                return None;
            }
            Some(match (left, right) {
                (true, true) => Align::Center,
                (true, false) => Align::Left,
                (false, true) => Align::Right,
                _ => Align::None,
            })
        })
        .collect()
}

/// Cells of a table row: split on unescaped pipes (inside code spans too,
/// as GFM does), with the optional leading and trailing pipe removed.
pub fn split_row(s: &str) -> Vec<String> {
    let s = s.trim();
    let s = s.strip_prefix('|').unwrap_or(s);
    let s = match s.strip_suffix('|') {
        Some(inner) if !inner.ends_with('\\') => inner,
        _ => s,
    };
    // A pipe after a backslash is escaped even when that backslash is itself
    // escaped, as both cmark-gfm and pulldown-cmark read it.
    let mut cells = Vec::new();
    let mut cell = String::new();
    for c in s.chars() {
        if c == '|' && !cell.ends_with('\\') {
            cells.push(cell.trim().to_string());
            cell.clear();
        } else {
            cell.push(c);
        }
    }
    cells.push(cell.trim().to_string());
    cells
}

// ---------------------------------------------------------------------------
// Parser state.

struct Open {
    container: Container,
    /// Columns a continuation line needs to stay inside (list items and
    /// footnotes); zero for quotes.
    indent: usize,
    leaves: usize,
    blank_before: Vec<usize>,
    /// A list item whose marker line was empty; a blank line next ends it.
    empty_start: bool,
}

enum LeafKind {
    Paragraph,
    Fence { fence: String, info: String, indent: usize },
    Math { brackets: bool },
    Html(HtmlEnd),
    IndentedCode,
    Table { align: Vec<Align>, rows: Vec<Vec<String>> },
}

struct Leaf {
    kind: LeafKind,
    /// Content lines (indentation already removed as the kind requires).
    lines: Vec<String>,
    /// Source lines with container prefixes stripped.
    raw: Vec<String>,
    lazy: Vec<(usize, usize)>,
    containers: Vec<Container>,
    blank_before: Vec<usize>,
}

#[derive(Default)]
struct Parser {
    open: Vec<Open>,
    leaf: Option<Leaf>,
    blocks: Vec<Block>,
    /// Blank lines since the last block, as container depths (see
    /// `Block::blank_before`).
    blanks: Vec<usize>,
    /// Per container depth, the last list item opened there, while its list
    /// can still be continued by a compatible marker.
    last_item: Vec<Option<(Id, ListMarker)>>,
    /// Whether `\[` opens a math block.
    latex: bool,
}

impl Parser {
    fn line(&mut self, line: &str) {
        let mut cur = Cur::new(line);

        // 1. Match open containers.
        let mut matched = 0;
        while matched < self.open.len() {
            let o = &self.open[matched];
            match o.container.kind {
                ContainerKind::Quote { .. } => {
                    if cur.indent() <= 3 && cur.peek_nonspace() == Some('>') {
                        cur.skip_indent();
                        cur.bump();
                        if cur.indent() > 0 {
                            cur.advance_cols(1);
                        }
                        matched += 1;
                    } else {
                        break;
                    }
                }
                ContainerKind::ListItem(_) | ContainerKind::Footnote { .. } => {
                    if cur.rest_is_blank() {
                        if o.empty_start && o.leaves == 0 {
                            break;
                        }
                        matched += 1;
                    } else if cur.indent() >= o.indent {
                        cur.advance_cols(o.indent);
                        matched += 1;
                    } else {
                        break;
                    }
                }
            }
        }
        let all_matched = matched == self.open.len();

        // 2. Verbatim leaves take every line while their containers continue.
        if all_matched && self.feed_verbatim(&cur) {
            return;
        }
        if !all_matched {
            let in_paragraph = matches!(self.leaf, Some(Leaf { kind: LeafKind::Paragraph, .. }));
            if in_paragraph && !cur.rest_is_blank() && !self.starts_block(&cur) {
                // Lazy continuation line.
                let leaf = self.leaf.as_mut().unwrap();
                leaf.lazy.push((leaf.raw.len(), matched));
                leaf.raw.push(cur.rest());
                leaf.lines.push(cur.content().to_string());
                return;
            }
            self.close_containers_from(matched);
        }

        // 3. Open new containers.
        let mut opened_new = false;
        loop {
            if cur.indent() > 3 {
                break;
            }
            let content = cur.content();
            if content.starts_with('>') {
                self.close_leaf();
                cur.skip_indent();
                cur.bump();
                if cur.indent() > 0 {
                    cur.advance_cols(1);
                }
                self.open_container(ContainerKind::Quote { padding: 1 }, 0);
                opened_new = true;
                continue;
            }
            if let Some(m) = list_marker(content) {
                if is_thematic_break(content) {
                    break;
                }
                let in_paragraph = !opened_new && matches!(self.leaf, Some(Leaf { kind: LeafKind::Paragraph, .. }));
                let starts_at_one = !matches!(m.marker, ListMarker::Ordered { start, .. } if start != 1);
                if in_paragraph && (m.empty || !starts_at_one) {
                    break;
                }
                self.close_leaf();
                let indent = cur.indent();
                cur.skip_indent();
                for _ in 0..m.width {
                    cur.bump();
                }
                let after = cur.indent();
                let padding = if m.empty || after >= 5 { 1 } else { after };
                if !m.empty {
                    cur.advance_cols(padding);
                }
                let mut task = None;
                let rest = cur.content();
                if !m.empty && cur.indent() == 0 {
                    for (box_, checked) in [("[ ]", false), ("[x]", true), ("[X]", true)] {
                        if let Some(after_box) = rest.strip_prefix(box_) {
                            if after_box.is_empty() || after_box.starts_with([' ', '\t']) {
                                task = Some(checked);
                                cur.bump();
                                cur.bump();
                                cur.bump();
                                if cur.indent() > 0 {
                                    cur.advance_cols(1);
                                }
                                break;
                            }
                        }
                    }
                }
                let depth = self.open.len();
                let list = match self.last_item.get(depth) {
                    Some(Some((id, prev))) if prev.compatible(&m.marker) => *id,
                    _ => next_id(),
                };
                let marker = m.marker.clone();
                self.open_container(
                    ContainerKind::ListItem(ListItem { list, marker: m.marker, indent, padding, task, loose: false }),
                    indent + m.width + padding,
                );
                self.open.last_mut().unwrap().empty_start = m.empty;
                if self.last_item.len() <= depth {
                    self.last_item.resize(depth + 1, None);
                }
                self.last_item[depth] = Some((list, marker));
                opened_new = true;
                continue;
            }
            if let Some(caps) = FOOTNOTE.captures(content) {
                let in_paragraph = !opened_new && matches!(self.leaf, Some(Leaf { kind: LeafKind::Paragraph, .. }));
                if in_paragraph {
                    break;
                }
                let label = caps[1].to_string();
                let len = caps[0].chars().count();
                self.close_leaf();
                cur.skip_indent();
                for _ in 0..len {
                    cur.bump();
                }
                if cur.indent() > 0 {
                    cur.advance_cols(1);
                }
                self.open_container(ContainerKind::Footnote { label }, 4);
                opened_new = true;
                continue;
            }
            break;
        }

        // 4. The leaf. A line holding only new container markers (`-`, `>`)
        // is not a blank line between blocks.
        if opened_new && cur.rest_is_blank() {
            return;
        }
        self.leaf_line(&cur);
    }

    /// Feed a line to an open fence, math or HTML block. Returns false when
    /// the line is not theirs.
    fn feed_verbatim(&mut self, cur: &Cur) -> bool {
        let Some(leaf) = self.leaf.as_mut() else { return false };
        match &leaf.kind {
            LeafKind::Fence { fence, indent, .. } => {
                if cur.indent() <= 3 && is_fence_close(cur.content(), fence) {
                    leaf.raw.push(cur.rest());
                    let fence = fence.clone();
                    self.close_leaf_with(|kind| match kind {
                        LeafKind::Fence { info, .. } => BlockKind::CodeFence { info, fence, closed: true },
                        _ => unreachable!(),
                    });
                } else {
                    let strip = cur.indent().min(*indent);
                    let mut c = Cur { s: cur.s, pos: cur.pos, col: cur.col, partial: cur.partial };
                    c.advance_cols(strip);
                    leaf.raw.push(cur.rest());
                    leaf.lines.push(c.rest());
                }
                true
            }
            LeafKind::Math { brackets } => {
                let brackets = *brackets;
                leaf.raw.push(cur.rest());
                if math_end(cur.content(), brackets) {
                    self.close_leaf_with(|_| BlockKind::Math { brackets });
                } else {
                    leaf.lines.push(cur.rest());
                }
                true
            }
            LeafKind::Html(end) => match *end {
                HtmlEnd::Contains(marker) => {
                    leaf.raw.push(cur.rest());
                    leaf.lines.push(cur.rest());
                    if cur.rest().to_ascii_lowercase().contains(marker) {
                        self.close_leaf();
                    }
                    true
                }
                HtmlEnd::Blank => {
                    if cur.rest_is_blank() {
                        self.close_leaf();
                        false
                    } else {
                        leaf.raw.push(cur.rest());
                        leaf.lines.push(cur.rest());
                        true
                    }
                }
            },
            _ => false,
        }
    }

    /// Whether the line (from the first unmatched container) starts a block.
    /// That rules out lazy paragraph continuation and ends a table. The
    /// paragraph-interruption limits do not apply here: the paragraph is not
    /// a child of the container the new block would open in.
    fn starts_block(&self, cur: &Cur) -> bool {
        if cur.indent() > 3 {
            return false;
        }
        let s = cur.content();
        s.starts_with('>')
            || is_thematic_break(s)
            || atx_heading(s).is_some()
            || fence_start(s).is_some()
            || s.trim_end() == "$$"
            || html_start(s, false).is_some()
            || list_marker(s).is_some()
            || FOOTNOTE.is_match(s)
    }

    fn leaf_line(&mut self, cur: &Cur) {
        if cur.rest_is_blank() {
            match self.leaf.as_ref().map(|l| &l.kind) {
                Some(LeafKind::IndentedCode) => {
                    // Spaces past the code indentation are content.
                    let mut c = Cur { s: cur.s, pos: cur.pos, col: cur.col, partial: cur.partial };
                    c.advance_cols(4);
                    let leaf = self.leaf.as_mut().unwrap();
                    leaf.lines.push(c.rest());
                    leaf.raw.push(cur.rest());
                    return;
                }
                Some(_) => self.close_leaf(),
                None => {}
            }
            self.blanks.push(self.open.len());
            return;
        }

        let indent = cur.indent();
        let content = cur.content();
        let in_paragraph = matches!(self.leaf, Some(Leaf { kind: LeafKind::Paragraph, .. }));

        if indent >= 4 {
            match self.leaf.as_ref().map(|l| &l.kind) {
                Some(LeafKind::IndentedCode) => {
                    let mut c = Cur { s: cur.s, pos: cur.pos, col: cur.col, partial: cur.partial };
                    c.advance_cols(4);
                    let leaf = self.leaf.as_mut().unwrap();
                    leaf.raw.push(cur.rest());
                    leaf.lines.push(c.rest());
                }
                Some(LeafKind::Paragraph) => {
                    let leaf = self.leaf.as_mut().unwrap();
                    leaf.raw.push(cur.rest());
                    leaf.lines.push(content.to_string());
                }
                Some(LeafKind::Table { .. }) => self.table_row(cur),
                _ => {
                    self.close_leaf();
                    let mut c = Cur { s: cur.s, pos: cur.pos, col: cur.col, partial: cur.partial };
                    c.advance_cols(4);
                    self.start_leaf(LeafKind::IndentedCode, vec![c.rest()], vec![cur.rest()]);
                }
            }
            return;
        }
        if matches!(self.leaf, Some(Leaf { kind: LeafKind::IndentedCode, .. })) {
            self.close_leaf();
        }

        if in_paragraph {
            let trimmed = content.trim_end();
            let setext = !trimmed.is_empty()
                && (trimmed.chars().all(|c| c == '=') || trimmed.chars().all(|c| c == '-'));
            if setext {
                let level = if trimmed.starts_with('=') { 1 } else { 2 };
                let underline = cur.rest();
                let leaf = self.leaf.as_mut().unwrap();
                leaf.raw.push(underline);
                let setext = Some(content.trim_end().to_string());
                self.close_leaf_with(|_| BlockKind::Heading { level, setext, closed: false });
                return;
            }
            if let Some(align) = delimiter_row(content) {
                let leaf = self.leaf.as_ref().unwrap();
                let header = split_row(leaf.lines.last().unwrap());
                if header.len() == align.len() && leaf.lines.last().unwrap().contains('|') {
                    self.start_table(header, align, cur.rest());
                    return;
                }
            }
        }

        if matches!(self.leaf, Some(Leaf { kind: LeafKind::Table { .. }, .. })) {
            if self.starts_block(cur) {
                self.close_leaf();
            } else {
                self.table_row(cur);
                return;
            }
        }

        if is_thematic_break(content) {
            self.close_leaf();
            self.emit_simple(BlockKind::ThematicBreak { marker: content.trim_end().to_string() }, String::new(), cur);
            return;
        }
        if let Some((level, text)) = atx_heading(content) {
            self.close_leaf();
            // Written with a closing sequence: kept when it is saved again.
            let body = content.trim_end().trim_start_matches('#');
            let closed = !text.is_empty() && body.trim_end_matches('#').len() != body.len()
                && body.trim_end_matches('#').ends_with([' ', '\t']);
            self.emit_simple(BlockKind::Heading { level, setext: None, closed }, text, cur);
            return;
        }
        if let Some((fence, info)) = fence_start(content) {
            self.close_leaf();
            self.start_leaf(LeafKind::Fence { fence, info, indent }, Vec::new(), vec![cur.rest()]);
            return;
        }
        if let Some(brackets) = math_start(content, self.latex) {
            self.close_leaf();
            self.start_leaf(LeafKind::Math { brackets }, Vec::new(), vec![cur.rest()]);
            return;
        }
        if let Some(end) = html_start(content, in_paragraph) {
            self.close_leaf();
            let text = cur.rest();
            self.start_leaf(LeafKind::Html(end), vec![text.clone()], vec![text.clone()]);
            if let HtmlEnd::Contains(marker) = end {
                if text.to_ascii_lowercase().contains(marker) {
                    self.close_leaf();
                }
            }
            return;
        }
        if in_paragraph {
            let leaf = self.leaf.as_mut().unwrap();
            leaf.raw.push(cur.rest());
            leaf.lines.push(content.to_string());
            return;
        }
        if content.trim().eq_ignore_ascii_case("[toc]") {
            self.emit_simple(BlockKind::Toc, content.trim().to_string(), cur);
            return;
        }
        if is_link_ref_def(content) {
            self.emit_simple(BlockKind::LinkRefDef, content.trim_end().to_string(), cur);
            return;
        }
        self.start_leaf(LeafKind::Paragraph, vec![content.to_string()], vec![cur.rest()]);
    }

    fn start_table(&mut self, header: Vec<String>, align: Vec<Align>, delimiter_raw: String) {
        let mut leaf = self.leaf.take().unwrap();
        let header_raw = leaf.raw.pop().unwrap();
        leaf.lines.pop();
        let raw_len = leaf.raw.len();
        leaf.lazy.retain(|(line, _)| *line < raw_len);
        let containers = leaf.containers.clone();
        let blank_before = if leaf.lines.is_empty() {
            std::mem::take(&mut leaf.blank_before)
        } else {
            self.leaf = Some(leaf);
            self.close_leaf();
            Vec::new()
        };
        self.leaf = Some(Leaf {
            kind: LeafKind::Table { align, rows: vec![header] },
            lines: Vec::new(),
            raw: vec![header_raw, delimiter_raw],
            lazy: Vec::new(),
            containers,
            blank_before,
        });
    }

    fn table_row(&mut self, cur: &Cur) {
        let leaf = self.leaf.as_mut().unwrap();
        if let LeafKind::Table { align, rows } = &mut leaf.kind {
            let mut cells = split_row(cur.content());
            cells.resize(align.len(), String::new());
            rows.push(cells);
        }
        leaf.raw.push(cur.rest());
    }

    fn open_container(&mut self, kind: ContainerKind, indent: usize) {
        let depth = self.open.len();
        if !matches!(kind, ContainerKind::ListItem(_)) && self.last_item.len() > depth {
            self.last_item.truncate(depth);
        }
        if self.last_item.len() > depth + 1 {
            self.last_item.truncate(depth + 1);
        }
        self.open.push(Open {
            container: Container::new(kind),
            indent,
            leaves: 0,
            blank_before: self.blanks.clone(),
            empty_start: false,
        });
    }

    fn start_leaf(&mut self, kind: LeafKind, lines: Vec<String>, raw: Vec<String>) {
        debug_assert!(self.leaf.is_none(), "a leaf must be closed before the next starts");
        let depth = self.open.len();
        self.last_item.truncate(depth);
        for o in &mut self.open {
            o.leaves += 1;
        }
        let containers = self.open.iter().map(|o| o.container.clone()).collect();
        let blank_before = std::mem::take(&mut self.blanks);
        self.leaf = Some(Leaf { kind, lines, raw, lazy: Vec::new(), containers, blank_before });
    }

    fn emit_simple(&mut self, kind: BlockKind, text: String, cur: &Cur) {
        self.start_leaf(LeafKind::Paragraph, Vec::new(), vec![cur.rest()]);
        self.close_leaf_with(move |_| kind);
        if let Some(b) = self.blocks.last_mut() {
            b.text = text;
        }
    }

    fn close_leaf(&mut self) {
        let Some(leaf) = self.leaf.as_ref() else { return };
        let kind = match &leaf.kind {
            LeafKind::Paragraph => BlockKind::Paragraph,
            LeafKind::Fence { fence, .. } => {
                let fence = fence.clone();
                return self.close_leaf_with(|kind| match kind {
                    LeafKind::Fence { info, .. } => BlockKind::CodeFence { info, fence, closed: false },
                    _ => unreachable!(),
                });
            }
            LeafKind::Math { brackets } => BlockKind::Math { brackets: *brackets },
            LeafKind::Html(_) => BlockKind::Html,
            LeafKind::IndentedCode => BlockKind::IndentedCode,
            LeafKind::Table { .. } => {
                return self.close_leaf_with(|kind| match kind {
                    LeafKind::Table { align, rows } => BlockKind::Table { align, rows },
                    _ => unreachable!(),
                });
            }
        };
        self.close_leaf_with(move |_| kind);
    }

    fn close_leaf_with(&mut self, make: impl FnOnce(LeafKind) -> BlockKind) {
        let Some(mut leaf) = self.leaf.take() else { return };
        if matches!(leaf.kind, LeafKind::IndentedCode) {
            // Trailing blank lines belong to the gap after the block.
            while leaf.lines.last().is_some_and(|l| l.trim().is_empty()) {
                leaf.lines.pop();
                leaf.raw.pop();
                self.blanks.push(leaf.containers.len());
            }
        }
        let kind = make(leaf.kind);
        let text = if let BlockKind::Heading { .. } = kind {
            leaf.lines.iter().map(|l| l.trim()).collect::<Vec<_>>().join("\n")
        } else {
            leaf.lines.join("\n")
        };
        let block = Block {
            id: next_id(),
            containers: leaf.containers,
            kind,
            text,
            blank_before: Some(leaf.blank_before),
            raw: Some(leaf.raw),
            lazy: leaf.lazy,
        };
        self.blocks.push(block);
    }

    /// Close the leaf and every container from `depth` inward. A container
    /// that never held a leaf (an empty list item, a bare `>`) gets an empty
    /// paragraph so it survives as an editable block.
    fn close_containers_from(&mut self, depth: usize) {
        self.close_leaf();
        while self.open.len() > depth {
            let o = self.open.last().unwrap();
            if o.leaves == 0 {
                let containers: Vec<Container> = self.open.iter().map(|o| o.container.clone()).collect();
                // The blank lines before the container's opening line are
                // still pending (no leaf took them); they are this block's.
                let n = o.blank_before.len().min(self.blanks.len());
                let blank_before: Vec<usize> = self.blanks.drain(..n).collect();
                for o in &mut self.open {
                    o.leaves += 1;
                }
                self.blocks.push(Block {
                    id: next_id(),
                    containers,
                    kind: BlockKind::Paragraph,
                    text: String::new(),
                    blank_before: Some(blank_before),
                    raw: Some(vec![String::new()]),
                    lazy: Vec::new(),
                });
            }
            self.open.pop();
        }
    }
}

/// A list is loose when blank lines separate two of its items, or two blocks
/// that sit directly inside one of its items.
fn mark_loose_lists(blocks: &mut [Block]) {
    let mut loose: HashSet<Id> = HashSet::new();
    for i in 1..blocks.len() {
        let (prev, cur) = (&blocks[i - 1], &blocks[i]);
        let Some(blanks) = &cur.blank_before else { continue };
        // A blank line written inside a quote that is nested in the item at
        // depth `d` (`  >`) belongs to that quote, not to the item.
        let separates = |d: usize| {
            blanks.iter().any(|&m| {
                let m = m.min(prev.containers.len());
                !prev.containers[(d + 1).min(m)..m].iter().any(|c| matches!(c.kind, ContainerKind::Quote { .. }))
            })
        };
        for d in 0..prev.containers.len().min(cur.containers.len()) {
            let (a, b) = (&prev.containers[d], &cur.containers[d]);
            if a.id != b.id {
                // The blank sits between two items of one list.
                if let (Some(x), Some(y)) = (a.list_item(), b.list_item()) {
                    if x.list == y.list && separates(d) {
                        loose.insert(x.list);
                    }
                }
                break;
            }
            let Some(item) = a.list_item() else { continue };
            // Same item: the blank separates two of its direct children unless
            // both sides are still inside one child, or are items of one
            // nested list (that list's own looseness is decided deeper).
            let same_child = match (prev.containers.get(d + 1), cur.containers.get(d + 1)) {
                (Some(p), Some(q)) => {
                    p.id == q.id
                        || matches!((p.list_item(), q.list_item()), (Some(x), Some(y)) if x.list == y.list)
                }
                _ => false,
            };
            if !same_child && separates(d) {
                loose.insert(item.list);
            }
        }
    }
    for block in blocks.iter_mut() {
        for c in &mut block.containers {
            if let ContainerKind::ListItem(item) = &mut c.kind {
                item.loose = loose.contains(&item.list);
            }
        }
    }
    detect_same_numbers(blocks);
}

/// Mark ordered lists written as `1. 1. 1.` so they serialize that way.
fn detect_same_numbers(blocks: &mut [Block]) {
    use std::collections::HashMap;
    // list id -> (first number, all equal so far, items seen)
    let mut lists: HashMap<Id, (u64, bool, HashSet<Id>)> = HashMap::new();
    for block in blocks.iter() {
        for c in &block.containers {
            if let ContainerKind::ListItem(ListItem { list, marker: ListMarker::Ordered { start, .. }, .. }) = &c.kind {
                let entry = lists.entry(*list).or_insert((*start, true, HashSet::new()));
                if entry.2.insert(c.id) && entry.2.len() > 1 && *start != entry.0 {
                    entry.1 = false;
                }
            }
        }
    }
    for block in blocks.iter_mut() {
        for c in &mut block.containers {
            if let ContainerKind::ListItem(ListItem { list, marker: ListMarker::Ordered { start, same_numbers, .. }, .. }) =
                &mut c.kind
            {
                let (first, same, items) = &lists[list];
                *start = *first;
                *same_numbers = *same && items.len() > 1;
            }
        }
    }
}

/// Whether an HTML block starting with this line may interrupt a paragraph
/// (every kind but the seventh, a lone open or close tag).
pub(crate) fn html_interrupts_paragraph(line: &str) -> bool {
    html_start(line, true).is_some()
}

/// Whether `line`, as the first line of a paragraph, would be read as some
/// other block.
fn starts_other_block(line: &str) -> bool {
    line.starts_with('>')
        || list_marker(line).is_some()
        || FOOTNOTE.is_match(line)
        || is_thematic_break(line)
        || atx_heading(line).is_some()
        || fence_start(line).is_some()
        || line.trim_end() == "$$"
        || html_start(line, false).is_some()
        || line.trim().eq_ignore_ascii_case("[toc]")
        || is_link_ref_def(line)
}

/// Whether `line`, following `prev` inside a paragraph, would end it or turn
/// it into a heading or table.
fn interrupts_paragraph(line: &str, prev: &str) -> bool {
    let t = line.trim_end();
    if !t.is_empty() && (t.chars().all(|c| c == '=') || t.chars().all(|c| c == '-')) {
        return true;
    }
    if let Some(align) = delimiter_row(line) {
        if prev.contains('|') && split_row(prev).len() == align.len() {
            return true;
        }
    }
    if let Some(m) = list_marker(line) {
        let starts_at_one = !matches!(m.marker, ListMarker::Ordered { start, .. } if start != 1);
        if !m.empty && starts_at_one {
            return true;
        }
    }
    line.starts_with('>')
        || is_thematic_break(line)
        || atx_heading(line).is_some()
        || fence_start(line).is_some()
        || t == "$$"
        || html_start(line, true).is_some()
}

/// Paragraph text as markdown lines: leading whitespace (which markdown
/// drops anyway) removed, and a backslash before any line that would
/// otherwise be read as a different block.
pub(crate) fn paragraph_lines(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.split('\n') {
        let line = line.trim_start_matches([' ', '\t']);
        let escape = match out.last() {
            None => starts_other_block(line),
            Some(prev) => interrupts_paragraph(line, prev),
        };
        out.push(if escape { escape_block_start(line) } else { line.to_string() });
    }
    // Spaces at the very end mean nothing to markdown (there is no line for
    // them to break), so they are not written.
    if let Some(last) = out.last_mut() {
        last.truncate(last.trim_end_matches([' ', '\t']).len());
    }
    out
}

/// Backslash-escape the character that makes a line start a block: the
/// first one, or the delimiter after an ordered list number.
fn escape_block_start(line: &str) -> String {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    format!("{}\\{}", &line[..digits], &line[digits..])
}

/// ATX heading text with a trailing run of `#` escaped, so it is not read as
/// the optional closing sequence.
pub(crate) fn atx_text(text: &str) -> String {
    let head = text.trim_end_matches('#');
    if head.len() < text.len() && (head.is_empty() || head.ends_with([' ', '\t'])) {
        format!("{head}\\{}", &text[head.len()..])
    } else {
        text.to_string()
    }
}

/// A table cell with its unescaped pipes escaped.
pub(crate) fn escape_cell(cell: &str) -> String {
    let mut out = String::with_capacity(cell.len());
    for c in cell.chars() {
        if c == '|' && !out.ends_with('\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// A one-line link reference definition (`[label]: /url "title"`).
fn is_link_ref_def(line: &str) -> bool {
    (!line.starts_with("[^") || line.starts_with("[^]")) && super::inline::parse_link_ref_def(line).is_some()
}

/// A line with full-width quote markers (`》`, `〉`) and task brackets
/// (`【 】`, `【x】`) as ASCII, as Typora reads them when it remaps
/// punctuation.
fn remap_line(line: &str) -> std::borrow::Cow<'_, str> {
    if !line.contains(['》', '〉', '【']) {
        return line.into();
    }
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    let indent = rest.len() - rest.trim_start_matches(' ').len();
    out.push_str(&rest[..indent]);
    rest = &rest[indent..];
    loop {
        if let Some(r) = rest.strip_prefix(['>', '》', '〉']) {
            out.push('>');
            rest = r;
            let spaces = rest.len() - rest.trim_start_matches(' ').len();
            out.push_str(&rest[..spaces]);
            rest = &rest[spaces..];
        } else {
            break;
        }
    }
    // `- 【 】 ` and `- 【x】 `.
    let marker = rest.len() - rest.trim_start_matches(['-', '*', '+']).len();
    if marker == 1 && rest[1..].starts_with(' ') {
        let after = rest[1..].trim_start_matches(' ');
        for (from, to) in [("【 】", "[ ]"), ("【x】", "[x]"), ("【X】", "[X]")] {
            if let Some(tail) = after.strip_prefix(from) {
                out.push_str(&rest[..rest.len() - after.len()]);
                out.push_str(to);
                out.push_str(tail);
                return out.into();
            }
        }
    }
    out.push_str(rest);
    out.into()
}
