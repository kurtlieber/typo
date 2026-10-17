//! Inline markdown → a tree of nodes with source ranges. Follows the
//! CommonMark inline algorithm (code spans, delimiter runs with the flanking
//! and rule-of-three conditions, link brackets, autolinks, raw HTML,
//! entities, breaks) plus Typora's extensions: `~~strike~~`, `==highlight==`,
//! `^sup^`, `~sub~`, `$math$`, `:emoji:`, `[^footnote]` references, bare URLs
//! and paired inline HTML tags such as `<u>…</u>`.
//!
//! Every node keeps its full source `range` and its `content` range. The
//! difference between the two is the node's syntax markers, which the editor
//! hides unless the caret is inside the node.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::imgtag::{self, ImgTag};
use super::model::{BlockKind, Document};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    /// `\*`: the content is the escaped character.
    Escape,
    /// `&amp;` and friends, with the decoded text.
    Entity(String),
    Code,
    Emph,
    Strong,
    Strike,
    Highlight,
    Sup,
    Sub,
    Math,
    Link { dest: String, title: String },
    Image { dest: String, title: String },
    /// `<img src=…>`, which Typora draws as an image too.
    ImgTag(ImgTag),
    /// `<https://…>`, `<a@b.c>` or a bare URL; the content is the URL as shown.
    Autolink { dest: String },
    FootnoteRef { label: String },
    /// One raw HTML tag; `closing` for `</x>`, `void` for `<br>` and `<x/>`.
    HtmlTag { name: String, closing: bool, void: bool },
    /// A processing instruction, declaration or CDATA section.
    Html,
    /// `<!-- … -->`.
    Comment,
    /// A matched pair of inline HTML tags; the tags are the markers.
    /// `color` and `background` are what the opening tag's `style` (or a
    /// `<font>`'s `color`) sets, as written.
    Tag { name: String, color: Option<String>, background: Option<String> },
    Emoji(&'static str),
    /// Punctuation shown smart (convert on rendering): `--` as `–`.
    Smart(char),
    SoftBreak,
    HardBreak,
}

impl Kind {
    /// An image, written `![…](…)` or as an `<img>` tag.
    pub fn is_image(&self) -> bool {
        matches!(self, Kind::Image { .. } | Kind::ImgTag(_))
    }

    /// An image's source, as written.
    pub fn image_src(&self) -> Option<&str> {
        match self {
            Kind::Image { dest, .. } => Some(dest),
            Kind::ImgTag(tag) => Some(&tag.src),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub kind: Kind,
    /// The whole construct in the source, markers included.
    pub range: Range<usize>,
    /// What remains visible when the markers are hidden.
    pub content: Range<usize>,
    pub children: Vec<Node>,
}

/// Typora's optional inline syntax (Preferences → Markdown). Typora ships
/// with inline math, subscript, superscript and highlight turned off, and
/// LaTeX's own delimiters, `\(…\)` and `\[…\]`, on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub math: bool,
    /// Typora's legacy inline math (compatible mode): `$` to the next `$`
    /// is math, whatever is around them.
    pub legacy_math: bool,
    pub latex_delimiters: bool,
    pub highlight: bool,
    pub superscript: bool,
    pub subscript: bool,
    pub emoji: bool,
    pub bare_urls: bool,
    /// Typora's link destinations with spaces in them, which Strict Mode
    /// turns off.
    pub loose_links: bool,
    /// GitHub's alerts (`> [!NOTE]`). Not inline, but syntax support all
    /// the same.
    pub alerts: bool,
    /// Smart punctuation converted on rendering.
    pub smart: Option<super::smart::Smart>,
    /// Curly quotes around a link's title read as straight ones (Remap
    /// Unicode Punctuation on Parse).
    pub remap: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            math: false,
            legacy_math: false,
            latex_delimiters: true,
            highlight: false,
            superscript: false,
            subscript: false,
            emoji: true,
            bare_urls: true,
            loose_links: true,
            alerts: true,
            smart: None,
            remap: false,
        }
    }
}

impl Options {
    /// Every extension on.
    pub fn all() -> Self {
        Options {
            math: true,
            legacy_math: false,
            latex_delimiters: true,
            highlight: true,
            superscript: true,
            subscript: true,
            emoji: true,
            bare_urls: true,
            loose_links: true,
            alerts: true,
            smart: None,
            remap: false,
        }
    }

    /// Plain CommonMark + GFM strikethrough, for comparing with other parsers.
    pub fn commonmark() -> Self {
        Options {
            math: false,
            legacy_math: false,
            latex_delimiters: false,
            highlight: false,
            superscript: false,
            subscript: false,
            emoji: false,
            bare_urls: false,
            loose_links: false,
            alerts: false,
            smart: None,
            remap: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRef {
    pub dest: String,
    pub title: String,
}

/// Link reference definitions by normalized label.
pub type Refs = HashMap<String, LinkRef>;

/// Case-fold and collapse whitespace, so `[Foo  Bar]` matches `[foo bar]`.
pub fn normalize_label(label: &str) -> String {
    // Lowercasing the uppercase approximates Unicode case folding, except
    // for the sharp s, which folds to "ss".
    label.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase().to_lowercase().replace('ß', "ss")
}

/// Parse `[label]: destination "title"`.
pub fn parse_link_ref_def(line: &str) -> Option<(String, LinkRef)> {
    let (label, end) = link_label(line, 0).filter(|(l, _)| !l.trim().is_empty())?;
    let rest = line[end..].strip_prefix(':')?;
    let start = line.len() - rest.len();
    let (end, dest, title) = dest_and_title(line, start, false, false)?;
    if !line[end..].trim().is_empty() {
        return None;
    }
    Some((normalize_label(&label), LinkRef { dest, title }))
}

/// Where the parts of a link reference definition are: the label (inside
/// its brackets), the destination and the title (with its quotes, empty
/// when there is none).
pub fn link_ref_def_parts(line: &str) -> Option<[Range<usize>; 3]> {
    parse_link_ref_def(line)?;
    let (_, end) = link_label(line, 0)?;
    let space = |at: usize| at + line[at..].len() - line[at..].trim_start().len();
    let dest_start = space(end + 1);
    let dest_end = if line[dest_start..].starts_with('<') {
        line[dest_start..].find('>').map_or(line.len(), |i| dest_start + i + 1)
    } else {
        line[dest_start..].find(char::is_whitespace).map_or(line.len(), |i| dest_start + i)
    };
    let title_start = space(dest_end);
    let title_end = title_start + line[title_start..].trim_end().len();
    Some([1..end - 1, dest_start..dest_end, title_start..title_end])
}

/// The document's link reference definitions; the first one for a label wins.
pub fn refs_of(doc: &Document) -> Refs {
    let mut refs = Refs::new();
    for b in &doc.blocks {
        if let BlockKind::LinkRefDef = b.kind {
            if let Some((label, r)) = parse_link_ref_def(&b.text) {
                refs.entry(label).or_insert(r);
            }
        }
    }
    refs
}

/// The image `src` consists of, with nothing but space around it; Typora
/// shows such a paragraph as the image, centred on its own.
pub fn lone_image<'a>(src: &str, nodes: &'a [Node]) -> Option<&'a Node> {
    let mut found = nodes.iter().filter(|n| n.kind != Kind::Text);
    let n = found.next().filter(|n| n.kind.is_image())?;
    let alone = found.next().is_none() && src[..n.range.start].trim().is_empty() && src[n.range.end..].trim().is_empty();
    alone.then_some(n)
}

/// The `<br>` tags in `nodes`, which break the line.
pub fn brs(nodes: &[Node]) -> Vec<&Node> {
    let mut out = Vec::new();
    for n in nodes {
        match &n.kind {
            Kind::HtmlTag { name, .. } if name.eq_ignore_ascii_case("br") => out.push(n),
            _ => out.extend(brs(&n.children)),
        }
    }
    out
}

/// `src` as the editor's text box holds it, at the same UTF-16 positions:
/// each `<br>`'s `>` a line break, so the line breaks there as in Typora,
/// and the rest as Qt hands text back (line and paragraph separators as
/// line breaks, no-break spaces as spaces), so that it hands back this.
pub fn boxed(src: &str, nodes: &[Node]) -> String {
    let ends: Vec<usize> = brs(nodes).iter().map(|n| n.range.end - 1).collect();
    src.char_indices()
        .map(|(i, c)| match c {
            '>' if ends.contains(&i) => '\n',
            '\u{2028}' | '\u{2029}' => '\n',
            '\u{a0}' => ' ',
            c => c,
        })
        .collect()
}

/// `src` after its box, showing it as `seen` (`boxed`), came to hold
/// `now`: what changed is taken from `now`, the rest from `src`.
pub fn unboxed(src: &str, seen: &str, now: &str) -> String {
    let (s, n): (Vec<char>, Vec<char>) = (seen.chars().collect(), now.chars().collect());
    let old: Vec<char> = src.chars().collect();
    if old.len() != s.len() {
        return now.to_string();
    }
    let a = s.iter().zip(&n).take_while(|(x, y)| x == y).count();
    let z = s[a..].iter().rev().zip(n[a..].iter().rev()).take_while(|(x, y)| x == y).count();
    old[..a].iter().chain(&n[a..n.len() - z]).chain(&old[old.len() - z..]).collect()
}

/// The markdown a box holding `now` shows, given `src`, the block's text
/// as last known, which may be a step behind: where `now` has what
/// `boxed` makes of `src`'s character at the same place, that character.
pub fn shown_source(now: &str, src: &str) -> String {
    if now.chars().count() != src.chars().count() {
        return now.to_string();
    }
    now.chars()
        .zip(src.chars())
        .map(|(n, s)| match (n, s) {
            ('\n', '>' | '\u{2028}' | '\u{2029}') | (' ', '\u{a0}') => s,
            _ => n,
        })
        .collect()
}

pub fn parse(src: &str, refs: &Refs, opts: &Options) -> Vec<Node> {
    let mut p = Parser {
        src,
        refs,
        opts,
        pos: 0,
        text_start: 0,
        arena: Vec::new(),
        head: None,
        tail: None,
        delims: Vec::new(),
        brackets: Vec::new(),
    };
    p.run();
    let nodes = pair_tags(p.collect(p.head), src);
    match &opts.smart {
        Some(smart) if smart.any() => super::smart::split(src, nodes, smart),
        _ => nodes,
    }
}

/// The plain text of nodes (for image alt text, outline titles, word counts).
pub fn plain_text(src: &str, nodes: &[Node], out: &mut String) {
    for n in nodes {
        match &n.kind {
            Kind::Text | Kind::Escape | Kind::Code | Kind::Math | Kind::Autolink { .. } => {
                out.push_str(&src[n.content.clone()])
            }
            Kind::Entity(s) => out.push_str(s),
            Kind::Emoji(e) => out.push_str(e),
            Kind::Smart(c) => out.push(*c),
            Kind::SoftBreak | Kind::HardBreak => out.push(' '),
            Kind::HtmlTag { .. } | Kind::Html | Kind::Comment | Kind::FootnoteRef { .. } => {}
            _ => plain_text(src, &n.children, out),
        }
    }
}

// ---------------------------------------------------------------------------
// The parser works on a linked list of nodes in an arena, so that emphasis
// and links can wrap a run of siblings without moving anything.

enum Item {
    Node(Kind),
    Delim { ch: char, len: usize, orig: usize, can_open: bool, can_close: bool },
    Bracket,
}

struct ANode {
    item: Item,
    range: Range<usize>,
    content: Range<usize>,
    prev: Option<usize>,
    next: Option<usize>,
    first: Option<usize>,
}

struct Bracket {
    node: usize,
    image: bool,
    /// Delimiter stack height when the bracket opened.
    delims: usize,
    active: bool,
}

struct Parser<'a> {
    src: &'a str,
    refs: &'a Refs,
    opts: &'a Options,
    pos: usize,
    /// Start of the pending text run.
    text_start: usize,
    arena: Vec<ANode>,
    head: Option<usize>,
    tail: Option<usize>,
    /// Arena indexes of delimiter runs that may still pair, in order.
    delims: Vec<usize>,
    brackets: Vec<Bracket>,
}

static AUTOLINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^<([A-Za-z][A-Za-z0-9+.\-]{1,31}:[^\x00-\x20<>]*)>").unwrap());
static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^<([a-zA-Z0-9.!#$%&'*+/=?^_`{|}~\-]+@[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*)>").unwrap()
});
static OPEN_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^<([A-Za-z][A-Za-z0-9\-]*)(?:\s+[A-Za-z_:][A-Za-z0-9_.:\-]*(?:\s*=\s*(?:[^\s"'=<>`]+|'[^']*'|"[^"]*"))?)*\s*(/?)>"#).unwrap()
});
static CLOSE_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^</([A-Za-z][A-Za-z0-9\-]*)\s*>").unwrap());
static OTHER_HTML: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:<!-->|<!--->|<!--(?s:.)*?-->|<\?(?s:.)*?\?>|<![A-Za-z][^>]*>|<!\[CDATA\[(?s:.)*?\]\]>)").unwrap()
});
static ENTITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^&(?:#[xX]([0-9a-fA-F]{1,6})|#([0-9]{1,7})|([A-Za-z][A-Za-z0-9]{1,31}));").unwrap());
static EMOJI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^:([a-z0-9_+\-]+):").unwrap());
static BARE_URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:https?://|www\.)[^\s<]+").unwrap());

const VOID_TAGS: &[&str] =
    &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];

fn is_punct(c: char) -> bool {
    c.is_ascii_punctuation() || (!c.is_ascii() && !c.is_alphanumeric() && !c.is_whitespace())
}

impl<'a> Parser<'a> {
    fn run(&mut self) {
        let bytes = self.src.as_bytes();
        while self.pos < bytes.len() {
            let c = bytes[self.pos];
            let handled = match c {
                b'\\' => self.backslash(),
                b'`' => self.code_span(),
                b'$' if self.opts.math => self.math(),
                b'*' | b'_' | b'~' => self.delim_run(c as char),
                b'=' if self.opts.highlight => self.delim_run('='),
                b'^' if self.opts.superscript => self.delim_run('^'),
                b'[' => self.open_bracket(false),
                b'!' if bytes.get(self.pos + 1) == Some(&b'[') => self.open_bracket(true),
                b']' => self.close_bracket(),
                b'<' => self.angle(),
                b'&' => self.entity(),
                b'\n' => self.newline(),
                b':' if self.opts.emoji => self.emoji(),
                b'h' | b'w' if self.opts.bare_urls => self.bare_url(),
                _ => false,
            };
            if !handled {
                self.pos += self.src[self.pos..].chars().next().map_or(1, char::len_utf8);
            }
        }
        // Trailing spaces end the paragraph; they are not text.
        let end = self.src.trim_end_matches([' ', '\t']).len().max(self.text_start);
        self.flush_text(end);
        self.process_emphasis(0);
    }

    // -- list plumbing ------------------------------------------------------

    fn alloc(&mut self, item: Item, range: Range<usize>, content: Range<usize>) -> usize {
        self.arena.push(ANode { item, range, content, prev: None, next: None, first: None });
        self.arena.len() - 1
    }

    fn append(&mut self, i: usize) {
        self.arena[i].prev = self.tail;
        match self.tail {
            Some(t) => self.arena[t].next = Some(i),
            None => self.head = Some(i),
        }
        self.tail = Some(i);
    }

    fn unlink(&mut self, i: usize) {
        let (prev, next) = (self.arena[i].prev, self.arena[i].next);
        match prev {
            Some(p) => self.arena[p].next = next,
            None => self.head = next,
        }
        match next {
            Some(n) => self.arena[n].prev = prev,
            None => self.tail = prev,
        }
    }

    fn flush_text(&mut self, upto: usize) {
        if upto > self.text_start {
            let r = self.text_start..upto;
            let i = self.alloc(Item::Node(Kind::Text), r.clone(), r);
            self.append(i);
        }
        self.text_start = self.text_start.max(upto);
    }

    /// Append a node, flushing the text before it, and continue after it.
    fn emit(&mut self, item: Item, range: Range<usize>, content: Range<usize>) -> usize {
        self.flush_text(range.start);
        let end = range.end;
        let i = self.alloc(item, range, content);
        self.append(i);
        self.pos = end;
        self.text_start = end;
        i
    }

    fn char_before(&self, at: usize) -> char {
        self.src[..at].chars().next_back().unwrap_or('\n')
    }

    fn char_at(&self, at: usize) -> char {
        self.src[at..].chars().next().unwrap_or('\n')
    }

    // -- constructs ---------------------------------------------------------

    fn backslash(&mut self) -> bool {
        let next = self.src.as_bytes().get(self.pos + 1).copied();
        match next {
            Some(b'\n') => {
                let end = self.pos + 2 + self.leading_ws(self.pos + 2);
                self.emit(Item::Node(Kind::HardBreak), self.pos..end, end..end);
                true
            }
            Some(b'(' | b'[') if self.opts.latex_delimiters && self.latex_math() => true,
            Some(c) if c.is_ascii_punctuation() => {
                let p = self.pos;
                self.emit(Item::Node(Kind::Escape), p..p + 2, p + 1..p + 2);
                true
            }
            _ => false,
        }
    }

    /// `\(…\)` or `\[…\]` on one line, with something between.
    fn latex_math(&mut self) -> bool {
        let start = self.pos;
        let close = if self.src.as_bytes()[start + 1] == b'(' { "\\)" } else { "\\]" };
        let line = &self.src[start + 2..];
        let line = &line[..line.find('\n').unwrap_or(line.len())];
        // Lazily, as Typora's pattern: the first closer after one character.
        let Some(first) = line.char_indices().nth(1).map(|(i, _)| i) else { return false };
        let Some(end) = line[first..].find(close).map(|i| start + 2 + first + i) else { return false };
        self.emit(Item::Node(Kind::Math), start..end + 2, start + 2..end);
        true
    }

    fn leading_ws(&self, at: usize) -> usize {
        self.src[at..].len() - self.src[at..].trim_start_matches([' ', '\t']).len()
    }

    fn newline(&mut self) -> bool {
        let p = self.pos;
        let spaces = self.src[self.text_start..p].len() - self.src[self.text_start..p].trim_end_matches(' ').len();
        let end = p + 1 + self.leading_ws(p + 1);
        let kind = if spaces >= 2 { Kind::HardBreak } else { Kind::SoftBreak };
        self.emit(Item::Node(kind), p - spaces..end, end..end);
        true
    }

    fn run_len(&self, at: usize, c: u8) -> usize {
        self.src.as_bytes()[at..].iter().take_while(|&&b| b == c).count()
    }

    fn code_span(&mut self) -> bool {
        let start = self.pos;
        let n = self.run_len(start, b'`');
        let mut i = start + n;
        let bytes = self.src.as_bytes();
        while i < bytes.len() {
            if bytes[i] == b'`' {
                let m = self.run_len(i, b'`');
                if m == n {
                    self.emit(Item::Node(Kind::Code), start..i + m, start + n..i);
                    return true;
                }
                i += m;
            } else {
                i += 1;
            }
        }
        // No closing run: the backticks are literal text.
        self.pos += n;
        true
    }

    /// `$…$` up to the next `$`: the content may not start or end with a
    /// space, and the closing `$` may not be followed by a digit (so `$5
    /// and $6` is text), unless parsing as legacy Typora did.
    fn math(&mut self) -> bool {
        let start = self.pos;
        let legacy = self.opts.legacy_math;
        if self.run_len(start, b'$') != 1 || !legacy && self.char_at(start + 1).is_whitespace() {
            return false;
        }
        let bytes = self.src.as_bytes();
        let mut i = start + 1;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i += 2,
                b'$' => {
                    let ok = legacy || !self.char_before(i).is_whitespace() && !self.char_at(i + 1).is_ascii_digit();
                    if !ok || i == start + 1 {
                        return false;
                    }
                    self.emit(Item::Node(Kind::Math), start..i + 1, start + 1..i);
                    return true;
                }
                _ => i += 1,
            }
        }
        false
    }

    fn delim_run(&mut self, ch: char) -> bool {
        let start = self.pos;
        let len = self.run_len(start, ch as u8);
        let valid = match ch {
            '~' => len <= 2,
            '=' => len == 2,
            '^' => len == 1,
            _ => true,
        };
        if !valid {
            self.pos += len;
            return true;
        }
        let end = start + len;
        let before = self.char_before(start);
        let after = self.char_at(end);
        let left = !after.is_whitespace() && (!is_punct(after) || before.is_whitespace() || is_punct(before));
        let right = !before.is_whitespace() && (!is_punct(before) || after.is_whitespace() || is_punct(after));
        let (can_open, can_close) = if ch == '_' {
            (left && (!right || is_punct(before)), right && (!left || is_punct(after)))
        } else {
            (left, right)
        };
        let i = self.emit(Item::Delim { ch, len, orig: len, can_open, can_close }, start..end, start..end);
        if can_open || can_close {
            self.delims.push(i);
        }
        true
    }

    fn open_bracket(&mut self, image: bool) -> bool {
        let start = self.pos;
        let len = if image { 2 } else { 1 };
        let i = self.emit(Item::Bracket, start..start + len, start + len..start + len);
        self.brackets.push(Bracket { node: i, image, delims: self.delims.len(), active: true });
        true
    }

    fn close_bracket(&mut self) -> bool {
        let Some(b) = self.brackets.last() else { return false };
        if !b.active {
            self.brackets.pop();
            return false;
        }
        let (opener, image, bottom) = (b.node, b.image, b.delims);
        let text = self.arena[opener].range.end..self.pos;
        let after = self.pos + 1;
        let label_text = &self.src[text.clone()];

        let footnote = !image
            && label_text.len() > 1
            && label_text.starts_with('^')
            && !label_text.contains(char::is_whitespace)
            && !self.src[after..].starts_with('(');
        let found = if footnote {
            Some((after, Kind::FootnoteRef { label: label_text[1..].to_string() }))
        } else {
            self.link_tail(after, label_text).map(|(end, dest, title)| {
                let kind = if image { Kind::Image { dest, title } } else { Kind::Link { dest, title } };
                (end, kind)
            })
        };
        let Some((end, kind)) = found else {
            self.brackets.pop();
            return false;
        };

        self.flush_text(self.pos);
        // A footnote reference shows its label, without the `^`.
        let content = if footnote { text.start + 1..text.end } else { text };
        if footnote {
            self.delims.truncate(bottom);
        } else {
            self.process_emphasis(bottom);
        }
        let range = self.arena[opener].range.start..end;
        // A footnote reference is a leaf: its label is not parsed.
        let first = if footnote { None } else { self.arena[opener].next };
        let node = self.alloc(Item::Node(kind), range, content);
        self.arena[node].first = first;
        if let Some(f) = first {
            self.arena[f].prev = None;
        }
        // The new node takes the opener's place at the end of the list.
        let prev = self.arena[opener].prev;
        self.arena[node].prev = prev;
        match prev {
            Some(p) => self.arena[p].next = Some(node),
            None => self.head = Some(node),
        }
        self.tail = Some(node);
        self.brackets.pop();
        if !image && !footnote {
            // No links inside links.
            for b in &mut self.brackets {
                if !b.image {
                    b.active = false;
                }
            }
        }
        self.pos = end;
        self.text_start = end;
        true
    }

    /// What follows a `]`: an inline destination, a reference label, or
    /// nothing (a shortcut reference). Returns the end and the target.
    fn link_tail(&self, after: usize, text: &str) -> Option<(usize, String, String)> {
        if self.src[after..].starts_with('(') {
            if let Some((end, dest, title)) = dest_and_title(self.src, after + 1, true, self.opts.remap) {
                let rest = &self.src[end..];
                let ws = rest.len() - rest.trim_start().len();
                if self.src[end + ws..].starts_with(')') {
                    return Some((end + ws + 1, dest, title));
                }
            }
            if let Some(found) = loose_dest_and_title(self.src, after + 1).filter(|_| self.opts.loose_links) {
                return Some(found);
            }
        }
        let lookup = |label: &str| self.refs.get(&normalize_label(label)).cloned();
        // `[^x]` after a link is a footnote reference, not its label.
        let footnote_next = self.src[after..].starts_with("[^");
        if let Some((label, end)) = link_label(self.src, after).filter(|_| !footnote_next) {
            // `[text][label]`, or `[text][]` with the text as the label.
            let label = if label.trim().is_empty() { text } else { &label };
            return lookup(label).map(|r| (end, r.dest, r.title));
        }
        if self.src[after..].starts_with("[]") {
            return lookup(text).map(|r| (after + 2, r.dest, r.title));
        }
        if text.len() > 999 || text.trim().is_empty() {
            return None;
        }
        lookup(text).map(|r| (after, r.dest, r.title))
    }

    fn angle(&mut self) -> bool {
        let start = self.pos;
        let rest = &self.src[start..];
        if let Some(c) = AUTOLINK.captures(rest) {
            let url = c.get(1).unwrap();
            let dest = c[1].to_string();
            self.emit(Item::Node(Kind::Autolink { dest }), start..start + c[0].len(), start + url.start()..start + url.end());
            return true;
        }
        if let Some(c) = EMAIL.captures(rest) {
            let addr = c.get(1).unwrap();
            let dest = format!("mailto:{}", &c[1]);
            self.emit(Item::Node(Kind::Autolink { dest }), start..start + c[0].len(), start + addr.start()..start + addr.end());
            return true;
        }
        if let Some(c) = OPEN_TAG.captures(rest) {
            let name = c[1].to_ascii_lowercase();
            let void = !c[2].is_empty() || VOID_TAGS.contains(&name.as_str());
            let end = start + c[0].len();
            if let Some(tag) = (name == "img").then(|| imgtag::parse(&c[0])).flatten() {
                self.emit(Item::Node(Kind::ImgTag(tag)), start..end, start..end);
                return true;
            }
            self.emit(Item::Node(Kind::HtmlTag { name, closing: false, void }), start..end, start..end);
            return true;
        }
        if let Some(c) = CLOSE_TAG.captures(rest) {
            let name = c[1].to_ascii_lowercase();
            let end = start + c[0].len();
            self.emit(Item::Node(Kind::HtmlTag { name, closing: true, void: false }), start..end, start..end);
            return true;
        }
        if let Some(m) = OTHER_HTML.find(rest) {
            let end = start + m.end();
            let kind = if m.as_str().starts_with("<!--") { Kind::Comment } else { Kind::Html };
            self.emit(Item::Node(kind), start..end, start..end);
            return true;
        }
        false
    }

    fn entity(&mut self) -> bool {
        let start = self.pos;
        let Some(c) = ENTITY.captures(&self.src[start..]) else { return false };
        let decoded = if let Some(hex) = c.get(1) {
            char_from_code(u32::from_str_radix(hex.as_str(), 16).unwrap_or(0)).to_string()
        } else if let Some(dec) = c.get(2) {
            char_from_code(dec.as_str().parse().unwrap_or(0)).to_string()
        } else {
            let decoded = html_escape::decode_html_entities(&c[0]);
            if decoded == &c[0] {
                return false;
            }
            decoded.into_owned()
        };
        let end = start + c[0].len();
        self.emit(Item::Node(Kind::Entity(decoded)), start..end, start..end);
        true
    }

    fn emoji(&mut self) -> bool {
        let start = self.pos;
        let Some(c) = EMOJI.captures(&self.src[start..]) else { return false };
        let Some(e) = emojis::get_by_shortcode(&c[1]) else { return false };
        let end = start + c[0].len();
        self.emit(Item::Node(Kind::Emoji(e.as_str())), start..end, start + 1..end - 1);
        true
    }

    /// GFM extended autolinks: `https://…` and `www.…` at a word start.
    fn bare_url(&mut self) -> bool {
        let start = self.pos;
        let before = self.char_before(start);
        if !(before.is_whitespace() || matches!(before, '*' | '_' | '~' | '(')) {
            return false;
        }
        let Some(m) = BARE_URL.find(&self.src[start..]) else { return false };
        let mut url = m.as_str();
        // Trailing punctuation is not part of the link, nor is an unbalanced `)`.
        loop {
            let trimmed = url.trim_end_matches(['?', '!', '.', ',', ':', '*', '_', '~', '\'', '"']);
            let trimmed = if trimmed.ends_with(')') && trimmed.matches('(').count() < trimmed.matches(')').count() {
                &trimmed[..trimmed.len() - 1]
            } else {
                trimmed
            };
            if trimmed.len() == url.len() {
                break;
            }
            url = trimmed;
        }
        if url.starts_with("www.") && !url[4..].contains('.') && url.len() <= 4 {
            return false;
        }
        let dest = if url.starts_with("www.") { format!("http://{url}") } else { url.to_string() };
        let end = start + url.len();
        self.emit(Item::Node(Kind::Autolink { dest }), start..end, start..end);
        true
    }

    // -- emphasis -----------------------------------------------------------

    fn delim(&self, i: usize) -> (char, usize, usize, bool, bool) {
        match self.arena[i].item {
            Item::Delim { ch, len, orig, can_open, can_close } => (ch, len, orig, can_open, can_close),
            _ => unreachable!(),
        }
    }

    fn set_len(&mut self, i: usize, new_len: usize) {
        if let Item::Delim { len, .. } = &mut self.arena[i].item {
            *len = new_len;
        }
    }

    /// The CommonMark "process emphasis" procedure over the delimiter stack
    /// above `bottom`.
    fn process_emphasis(&mut self, bottom: usize) {
        let mut ci = bottom;
        while ci < self.delims.len() {
            let c = self.delims[ci];
            let (ch, c_len, c_orig, c_open, c_close) = self.delim(c);
            if !c_close {
                ci += 1;
                continue;
            }
            let mut found = None;
            let mut oi = ci;
            while oi > bottom {
                oi -= 1;
                let o = self.delims[oi];
                let (och, o_len, o_orig, o_open, o_close) = self.delim(o);
                if och != ch || !o_open {
                    continue;
                }
                let used = match ch {
                    '*' | '_' => {
                        let odd_match = (o_close || c_open) && (o_orig + c_orig) % 3 == 0 && !(o_orig % 3 == 0 && c_orig % 3 == 0);
                        if odd_match {
                            continue;
                        }
                        if o_len >= 2 && c_len >= 2 { 2 } else { 1 }
                    }
                    '~' | '=' | '^' if o_len == c_len => c_len,
                    _ => continue,
                };
                // Typora's sub/superscript holds no unescaped spaces
                // (`X~long\ text~`); a single `~` pair with spaces is a strike.
                let spaced = || has_unescaped_space(&self.src[self.arena[o].range.end..self.arena[c].range.start]);
                if ch == '^' && spaced() {
                    continue;
                }
                let sub = ch == '~' && used == 1 && self.opts.subscript && !spaced();
                found = Some((oi, used, sub));
                break;
            }
            let Some((oi, used, sub)) = found else {
                if !c_open {
                    self.delims.remove(ci);
                } else {
                    ci += 1;
                }
                continue;
            };
            let o = self.delims[oi];
            let kind = match (ch, used) {
                ('*' | '_', 2) => Kind::Strong,
                ('*' | '_', _) => Kind::Emph,
                ('~', _) if sub => Kind::Sub,
                ('~', _) => Kind::Strike,
                ('=', _) => Kind::Highlight,
                _ => Kind::Sup,
            };
            // Delimiters between the pair can no longer match.
            self.delims.drain(oi + 1..ci);
            ci = oi + 1;
            self.wrap(o, c, kind, used);
            let o_left = self.delim(o).1;
            let c_left = self.delim(c).1;
            if o_left == 0 {
                self.unlink(o);
                self.delims.remove(oi);
                ci -= 1;
            }
            if c_left == 0 {
                self.unlink(c);
                self.delims.remove(ci);
            }
        }
        self.delims.truncate(bottom);
    }

    /// Wrap everything between opener `o` and closer `c` in a new node that
    /// takes `used` delimiter characters from each.
    fn wrap(&mut self, o: usize, c: usize, kind: Kind, used: usize) {
        let o_end = self.arena[o].range.end;
        let c_start = self.arena[c].range.start;
        let first = self.arena[o].next.filter(|&f| f != c);
        let last = self.arena[c].prev.filter(|&l| l != o);
        let node = self.alloc(Item::Node(kind), o_end - used..c_start + used, o_end..c_start);
        if let (Some(f), Some(l)) = (first, last) {
            self.arena[f].prev = None;
            self.arena[l].next = None;
            self.arena[node].first = Some(f);
        }
        self.arena[node].prev = Some(o);
        self.arena[node].next = Some(c);
        self.arena[o].next = Some(node);
        self.arena[c].prev = Some(node);
        let o_len = self.delim(o).1 - used;
        self.set_len(o, o_len);
        self.arena[o].range.end -= used;
        self.arena[o].content = self.arena[o].range.clone();
        let c_len = self.delim(c).1 - used;
        self.set_len(c, c_len);
        self.arena[c].range.start += used;
        self.arena[c].content = self.arena[c].range.clone();
    }

    // -- output -------------------------------------------------------------

    fn collect(&self, first: Option<usize>) -> Vec<Node> {
        let mut out: Vec<Node> = Vec::new();
        let mut cur = first;
        while let Some(i) = cur {
            let n = &self.arena[i];
            match &n.item {
                Item::Node(Kind::Text) | Item::Delim { .. } | Item::Bracket => {
                    let r = n.range.clone();
                    match out.last_mut() {
                        Some(last) if last.kind == Kind::Text && last.range.end == r.start => {
                            last.range.end = r.end;
                            last.content.end = r.end;
                        }
                        _ if r.is_empty() => {}
                        _ => out.push(Node { kind: Kind::Text, range: r.clone(), content: r, children: Vec::new() }),
                    }
                }
                Item::Node(kind) => out.push(Node {
                    kind: kind.clone(),
                    range: n.range.clone(),
                    content: n.content.clone(),
                    children: pair_tags(self.collect(n.first), self.src),
                }),
            }
            cur = n.next;
        }
        out
    }
}

/// Pair `<x>` with a later `</x>` among siblings into one `Tag` node.
fn pair_tags(nodes: Vec<Node>, src: &str) -> Vec<Node> {
    if !nodes.iter().any(|n| matches!(n.kind, Kind::HtmlTag { closing: true, .. })) {
        return nodes;
    }
    let mut out: Vec<Node> = Vec::new();
    let mut open: Vec<(usize, String)> = Vec::new();
    for node in nodes {
        match &node.kind {
            Kind::HtmlTag { name, closing: false, void: false } => {
                open.push((out.len(), name.clone()));
                out.push(node);
            }
            Kind::HtmlTag { name, closing: true, .. } => {
                if let Some(k) = open.iter().rposition(|(_, n)| n == name) {
                    let at = open[k].0;
                    open.truncate(k);
                    let children: Vec<Node> = out.drain(at + 1..).collect();
                    let start_tag = out.pop().unwrap();
                    let tag = &src[start_tag.range.clone()];
                    let color = imgtag::style_of(tag, "color").or_else(|| (name == "font").then(|| imgtag::attr(tag, "color")).flatten());
                    let background = imgtag::style_of(tag, "background-color").or_else(|| imgtag::style_of(tag, "background"));
                    out.push(Node {
                        kind: Kind::Tag { name: name.clone(), color, background },
                        range: start_tag.range.start..node.range.end,
                        content: start_tag.range.end..node.range.start,
                        children,
                    });
                } else {
                    out.push(node);
                }
            }
            _ => out.push(node),
        }
    }
    out
}

fn has_unescaped_space(s: &str) -> bool {
    let mut escaped = false;
    for c in s.chars() {
        if c.is_whitespace() && !escaped {
            return true;
        }
        escaped = c == '\\' && !escaped;
    }
    false
}

fn char_from_code(code: u32) -> char {
    if code == 0 { '\u{FFFD}' } else { char::from_u32(code).unwrap_or('\u{FFFD}') }
}

// ---------------------------------------------------------------------------
// Link pieces shared by inline links and reference definitions.

/// `[label]` at `at`: no unescaped brackets inside, at most 999 characters.
fn link_label(src: &str, at: usize) -> Option<(String, usize)> {
    let rest = src.get(at..)?.strip_prefix('[')?;
    let mut i = 0;
    let bytes = rest.as_bytes();
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'[' => return None,
            b']' => {
                let label = &rest[..i];
                if label.len() > 999 {
                    return None;
                }
                return Some((label.to_string(), at + 1 + i + 1));
            }
            _ => i += 1,
        }
    }
    None
}

/// A link destination and optional title starting at `at` (after `(` or
/// `:`), allowing whitespace and one line break between the parts.
/// `inline` allows an empty destination (`[a]()`).
fn dest_and_title(src: &str, at: usize, inline: bool, remap: bool) -> Option<(usize, String, String)> {
    let skip_ws = |i: usize| {
        let rest = &src[i..];
        let mut n = 0;
        let mut newlines = 0;
        for c in rest.chars() {
            match c {
                ' ' | '\t' => n += 1,
                '\n' if newlines == 0 => {
                    newlines += 1;
                    n += 1;
                }
                _ => break,
            }
        }
        i + n
    };
    let mut i = skip_ws(at);
    let bytes = src.as_bytes();
    let dest;
    if bytes.get(i) == Some(&b'<') {
        let mut j = i + 1;
        loop {
            match bytes.get(j)? {
                b'\\' => j += 2,
                b'>' => break,
                b'<' | b'\n' => return None,
                _ => j += 1,
            }
        }
        dest = unescape(&src[i + 1..j]);
        i = j + 1;
    } else {
        let mut j = i;
        let mut depth = 0usize;
        while let Some(&b) = bytes.get(j) {
            match b {
                b'\\' if bytes.get(j + 1).is_some_and(|c| c.is_ascii_punctuation()) => j += 2,
                b'(' => {
                    depth += 1;
                    j += 1;
                }
                b')' if depth == 0 => break,
                b')' => {
                    depth -= 1;
                    j += 1;
                }
                b if b <= b' ' => break,
                _ => j += 1,
            }
        }
        if depth != 0 || (j == i && !inline) {
            return None;
        }
        if j == i && !matches!(bytes.get(j), Some(b')')) {
            return None;
        }
        dest = unescape(&src[i..j]);
        i = j;
    }
    // Optional title, separated by whitespace.
    let before_title = i;
    let t = skip_ws(i);
    // Curly quotes, as Typora remaps them: any double quote ends a title
    // any double quote opened, and the same for single ones.
    if t > before_title && remap {
        const DOUBLE: &str = "«»“”„‟〝〟";
        const SINGLE: &str = "‚‘’‛‹›";
        if let Some(set) = src[t..].chars().next().and_then(|q| [DOUBLE, SINGLE].into_iter().find(|s| s.contains(q))) {
            let open = t + src[t..].chars().next().unwrap().len_utf8();
            if let Some(close) = src[open..].find(|c| set.contains(c)) {
                let close = open + close;
                let after = close + src[close..].chars().next().unwrap().len_utf8();
                return Some((after, dest, unescape(&src[open..close])));
            }
        }
    }
    if t > before_title {
        if let Some(&q) = bytes.get(t) {
            let close = match q {
                b'"' => Some(b'"'),
                b'\'' => Some(b'\''),
                b'(' => Some(b')'),
                _ => None,
            };
            if let Some(close) = close {
                let mut j = t + 1;
                while let Some(&b) = bytes.get(j) {
                    if b == b'\\' {
                        j += 2;
                        continue;
                    }
                    if b == close {
                        let title = unescape(&src[t + 1..j]);
                        return Some((j + 1, dest, title));
                    }
                    if q == b'(' && b == b'(' {
                        return None;
                    }
                    j += 1;
                }
                return None;
            }
        }
    }
    Some((i, dest, String::new()))
}

/// Typora's reading of an inline destination, where CommonMark's fails: a
/// destination with spaces in it (`![cat](my cat.png)`, as Typora inserts
/// images when it doesn't escape their paths), then an optional quoted
/// title, then `)`. `at` is just past the `(`; returns the end past the `)`.
fn loose_dest_and_title(src: &str, at: usize) -> Option<(usize, String, String)> {
    let bytes = src.as_bytes();
    let start = at + usize::from(bytes.get(at) == Some(&b'<'));
    // What may follow the destination: `>`, a title, then `)`.
    let tail = |j: usize| -> Option<(usize, String)> {
        let k = j + usize::from(bytes.get(j) == Some(&b'>'));
        let blanks = bytes[k..].iter().take_while(|b| matches!(b, b' ' | b'\t')).count();
        if blanks > 0 {
            let q = *bytes.get(k + blanks)?;
            if q == b'"' || q == b'\'' {
                let open = k + blanks + 1;
                for (i, _) in src[open..].match_indices(q as char) {
                    let close = open + i;
                    let after = close + 1 + bytes[close + 1..].iter().take_while(|b| matches!(b, b' ' | b'\t')).count();
                    if bytes.get(after) == Some(&b')') {
                        return Some((after + 1, unescape(&src[open..close])));
                    }
                }
            }
        }
        (bytes.get(k) == Some(&b')')).then(|| (k + 1, String::new()))
    };
    let mut j = start;
    loop {
        if let Some((end, title)) = tail(j) {
            return Some((end, unescape(src[start..j].trim()), title));
        }
        match *bytes.get(j)? {
            b'(' => j += 1 + src[j + 1..].find(')')? + 1,
            b')' => return None,
            _ => j += 1,
        }
    }
}

/// Resolve backslash escapes and entities (in destinations and titles).
pub fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < s.len() {
        if bytes[i] == b'\\' && bytes.get(i + 1).is_some_and(|c| c.is_ascii_punctuation()) {
            out.push(bytes[i + 1] as char);
            i += 2;
            continue;
        }
        if bytes[i] == b'&' {
            if let Some(c) = ENTITY.captures(&s[i..]) {
                let decoded = if let Some(hex) = c.get(1) {
                    Some(char_from_code(u32::from_str_radix(hex.as_str(), 16).unwrap_or(0)).to_string())
                } else if let Some(dec) = c.get(2) {
                    Some(char_from_code(dec.as_str().parse().unwrap_or(0)).to_string())
                } else {
                    let d = html_escape::decode_html_entities(&c[0]);
                    (d != &c[0]).then(|| d.into_owned())
                };
                if let Some(d) = decoded {
                    out.push_str(&d);
                    i += c[0].len();
                    continue;
                }
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}
