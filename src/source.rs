//! Source Code Mode's highlighting. The markdown is read a line at a time,
//! as CodeMirror's markdown mode reads it in Typora's source view. Whatever
//! a line leaves open (a code fence, a math block, front matter, an HTML
//! block, a list, a table) is carried to the next line as a `State`, which
//! the highlighter stores on the line. Each line becomes `Look` layers,
//! outer first, which `runs` flattens.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use crate::md::inline::{self, Kind, Node, Options, Refs};
use crate::md::parse::{self, HtmlEnd};

/// Colours by role. The highlighter takes them from the theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Header,
    Quote,
    Rule,
    Code,
    Math,
    Link,
    /// Link addresses, link titles and attribute values: CodeMirror's strings.
    Url,
    Tag,
    /// Escapes, entities, emoji and front matter keys.
    Atom,
}

impl Ink {
    pub const ALL: [Ink; 9] =
        [Ink::Header, Ink::Quote, Ink::Rule, Ink::Code, Ink::Math, Ink::Link, Ink::Url, Ink::Tag, Ink::Atom];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Look {
    pub ink: Option<Ink>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub mono: bool,
    /// Highlighted (`==mark==`) text, drawn on the mark colour.
    pub mark: bool,
    /// A header's level, because Typora draws the first two larger.
    pub level: u8,
}

impl Look {
    fn ink(ink: Ink) -> Look {
        Look { ink: Some(ink), ..Look::default() }
    }

    fn mono(mut self) -> Look {
        self.mono = true;
        self
    }

    /// Typora's code and comments: the code colour, in monospace.
    fn code() -> Look {
        Look::ink(Ink::Code).mono()
    }

    fn header(level: u8) -> Look {
        Look { bold: true, mono: true, level, ..Look::ink(Ink::Header) }
    }

    fn link() -> Look {
        Look { underline: true, ..Look::ink(Ink::Link) }
    }

    /// `top` laid over this look: its colour and size win, and the styles
    /// add up.
    fn over(self, top: Look) -> Look {
        Look {
            ink: top.ink.or(self.ink),
            bold: self.bold || top.bold,
            italic: self.italic || top.italic,
            underline: self.underline || top.underline,
            strike: self.strike || top.strike,
            mono: self.mono || top.mono,
            mark: self.mark || top.mark,
            level: if top.level > 0 { top.level } else { self.level },
        }
    }

    /// The font size against the text's: Typora's 2rem and 1.4rem headers.
    pub fn scale(&self) -> f64 {
        match self.level {
            1 => 2.0,
            2 => 1.4,
            _ => 1.0,
        }
    }
}

/// The block a line leaves open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Block {
    /// Nothing: the start, or after a blank line or a one-line block.
    #[default]
    Blank,
    /// A paragraph, which the next line may continue or underline.
    Para,
    Table,
    FrontMatter,
    Fence { tilde: bool, len: u8 },
    /// `$$`, or LaTeX's `\[` when `brackets`.
    Math { brackets: bool },
    /// An HTML block, until the `HTML_ENDS` marker it names (0: a blank
    /// line).
    Html(u8),
}

const HTML_ENDS: [&str; 9] = ["", "</script>", "</pre>", "</style>", "</textarea>", "-->", "?>", "]]>", ">"];
const COMMENT_END: u8 = 5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub block: Block,
    /// Inside a list, where indented lines continue an item instead of
    /// starting code.
    pub list: bool,
    /// In a fence in a language the highlighter knows, its parser's state
    /// after this line, numbered by `code::States` (0: none).
    pub code: u16,
}

impl State {
    /// As a line's state in a `QSyntaxHighlighter`, which keeps an int
    /// (and starts at -1).
    pub fn encode(self) -> i32 {
        let (kind, extra) = match self.block {
            Block::Blank => (0, 0),
            Block::Para => (1, 0),
            Block::Table => (2, 0),
            Block::FrontMatter => (3, 0),
            Block::Fence { tilde, len } => (4, (self.code as i32) << 7 | (tilde as i32) << 6 | len.min(63) as i32),
            Block::Math { brackets } => (5, brackets as i32),
            Block::Html(end) => (6, end as i32),
        };
        kind | (self.list as i32) << 4 | extra << 8
    }

    pub fn decode(n: i32) -> State {
        if n < 0 {
            return State::default();
        }
        let extra = n >> 8;
        let block = match n & 15 {
            1 => Block::Para,
            2 => Block::Table,
            3 => Block::FrontMatter,
            4 => Block::Fence { tilde: extra >> 6 & 1 == 1, len: (extra & 63) as u8 },
            5 => Block::Math { brackets: extra & 1 == 1 },
            6 => Block::Html((extra as usize).min(HTML_ENDS.len() - 1) as u8),
            _ => Block::Blank,
        };
        let code = if matches!(block, Block::Fence { .. }) { (extra >> 7 & 0xffff) as u16 } else { 0 };
        State { block, list: n >> 4 & 1 == 1, code }
    }
}

pub type Layers = Vec<(Range<usize>, Look)>;

/// Read one line of the source, given what the line before it left open.
/// `first` is the document's first line, where front matter may start;
/// `next` is the line after it, which may underline it as a header.
pub fn line(text: &str, prev: State, first: bool, next: &str, refs: &Refs, opts: &Options) -> (Layers, State) {
    let mut out = Vec::new();
    let all = 0..text.len();
    let list = prev.list;
    let stay = |block| State { block, list, code: 0 };
    match prev.block {
        Block::FrontMatter => {
            if matches!(text.trim_end(), "---" | "...") {
                out.push((all, Look::ink(Ink::Rule)));
                return (out, State::default());
            }
            front_matter(text, &mut out);
            return (out, prev);
        }
        Block::Fence { tilde, len } => {
            out.push((all, Look::code()));
            let at = quotes(text, &mut out);
            let fence = if tilde { "~" } else { "`" }.repeat(len as usize);
            let closed = parse::is_fence_close(text[at..].trim_start(), &fence);
            return (out, if closed { stay(Block::Blank) } else { prev });
        }
        Block::Math { brackets } => {
            out.push((all, Look::ink(Ink::Math).mono()));
            let at = quotes(text, &mut out);
            let closed = parse::math_end(&text[at..], brackets);
            return (out, if closed { stay(Block::Blank) } else { prev });
        }
        Block::Html(end) => {
            if end == 0 && text.trim().is_empty() {
                return (out, stay(Block::Blank));
            }
            if end == COMMENT_END {
                out.push((all, Look::code()));
            } else {
                tags(text, all, 0, &mut out);
            }
            let closed = end > 0 && text.to_ascii_lowercase().contains(HTML_ENDS[end as usize]);
            return (out, if closed { stay(Block::Blank) } else { prev });
        }
        _ => {}
    }
    if first && text.trim_end() == "---" {
        out.push((all, Look::ink(Ink::Rule)));
        return (out, State { block: Block::FrontMatter, ..State::default() });
    }
    if text.trim().is_empty() {
        return (out, stay(Block::Blank));
    }
    // Code indented four spaces, where no paragraph or list item goes on.
    if indent(text) >= 4 && prev.block == Block::Blank && !prev.list {
        out.push((all, Look::code()));
        return (out, stay(Block::Blank));
    }
    // A setext header's underline.
    if prev.block == Block::Para
        && let Some(level) = underline(text)
    {
        out.push((all, Look::header(level)));
        return (out, stay(Block::Blank));
    }

    let (at, listed) = markers(text, &mut out);
    let rest = &text[at..];
    let list = listed || (prev.list && (text.starts_with([' ', '\t']) || prev.block == Block::Para));
    let stay = |block| State { block, list, code: 0 };
    let body = at..text.len();

    if rest.is_empty() {
        return (out, stay(Block::Blank));
    }
    if parse::is_thematic_break(rest) {
        out.push((body, Look::ink(Ink::Rule)));
        return (out, stay(Block::Blank));
    }
    if let Some((level, _)) = parse::atx_heading(rest) {
        out.push((body, Look::header(level)));
        let hashes = rest.len() - rest.trim_start_matches('#').len();
        inlines(text, at + hashes, refs, opts, &mut out);
        return (out, stay(Block::Blank));
    }
    if let Some((fence, _)) = parse::fence_start(rest) {
        out.push((body, Look::code()));
        let tilde = fence.starts_with('~');
        // Longer fences close with 63 marks or more.
        return (out, stay(Block::Fence { tilde, len: fence.len().min(63) as u8 }));
    }
    if let Some(brackets) = parse::math_start(rest, opts.latex_delimiters) {
        out.push((body, Look::ink(Ink::Math).mono()));
        return (out, stay(Block::Math { brackets }));
    }
    if let Some(end) = parse::html_start(rest, prev.block == Block::Para) {
        let end = match end {
            HtmlEnd::Blank => 0,
            HtmlEnd::Contains(m) => HTML_ENDS.iter().position(|e| *e == m).unwrap_or(0) as u8,
        };
        if end == COMMENT_END {
            out.push((body, Look::code()));
        } else {
            tags(text, body, 0, &mut out);
        }
        // Kinds 1 to 5 may end on their first line; the marker that opens
        // a comment can't also close it.
        let skip = if end == COMMENT_END { 4 } else { 1 };
        let closed = end > 0 && rest.get(skip..).is_some_and(|r| r.to_ascii_lowercase().contains(HTML_ENDS[end as usize]));
        return (out, stay(if closed { Block::Blank } else { Block::Html(end) }));
    }
    // A footnote's label, then its text.
    if let Some(end) = footnote_label(rest) {
        out.push((at..at + end, Look::link()));
        inlines(text, at + end, refs, opts, &mut out);
        return (out, stay(Block::Para));
    }
    if prev.block != Block::Para {
        if let Some([label, dest, title]) = inline::link_ref_def_parts(rest) {
            out.push((at + label.start - 1..at + label.end + 1, Look::link()));
            out.push((at + dest.start..at + dest.end, Look::ink(Ink::Url)));
            if !title.is_empty() {
                out.push((at + title.start..at + title.end, Look::ink(Ink::Url)));
            }
            return (out, stay(Block::Blank));
        }
    }
    let cells = rest.contains('|');
    if cells && (prev.block == Block::Table || rest.starts_with('|') || parse::delimiter_row(rest).is_some()) {
        out.push((body, Look::default().mono()));
        inlines(text, at, refs, opts, &mut out);
        return (out, stay(Block::Table));
    }
    // The line a setext underline makes a header, as CodeMirror looks
    // ahead for it: this line only, even when the paragraph has more.
    if at == 0
        && let Some(level) = underline(next)
    {
        out.push((body, Look::header(level)));
    }
    inlines(text, at, refs, opts, &mut out);
    (out, stay(Block::Para))
}

/// A setext header's underline (after a paragraph line): its level.
fn underline(text: &str) -> Option<u8> {
    let t = text.trim();
    let all = |c| t.chars().all(|x| x == c);
    if indent(text) >= 4 || t.is_empty() {
        None
    } else if all('=') {
        Some(1)
    } else if all('-') {
        Some(2)
    } else {
        None
    }
}

/// Flatten layers into runs that don't overlap, each with every layer over
/// it laid on in order. Plain stretches are left out.
pub fn runs(len: usize, layers: &[(Range<usize>, Look)]) -> Vec<(Range<usize>, Look)> {
    let mut cuts: Vec<usize> = layers.iter().flat_map(|(r, _)| [r.start.min(len), r.end.min(len)]).collect();
    cuts.extend([0, len]);
    cuts.sort_unstable();
    cuts.dedup();
    let mut out: Vec<(Range<usize>, Look)> = Vec::new();
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let look = layers.iter().filter(|(r, _)| r.start <= a && b <= r.end).fold(Look::default(), |l, (_, t)| l.over(*t));
        if look == Look::default() {
            continue;
        }
        match out.last_mut() {
            Some((r, l)) if r.end == a && *l == look => r.end = b,
            _ => out.push((a..b, look)),
        }
    }
    out
}

/// CodeMirror's patterns for list items and quotes as Enter reads them:
/// the indent, the bullet (with its number and delimiter), and the space
/// after it; and a line holding nothing but one.
static ITEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\s*)(>[> ]*|[*+-] \[[x ]\]\s|[*+-]\s|([0-9]+)([.)]))(\s*)").unwrap());
static EMPTY_ITEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\s*)(>[> ]*|[*+-] \[[x ]\]|[*+-]|([0-9]+)[.)])(\s*)$").unwrap());

/// Source Code Mode's typing preferences, as Typora gives its CodeMirror
/// them: ordered items numbered alike, brackets paired, and the indent
/// unit (and tab width).
#[derive(Clone, Copy, Debug)]
pub struct Typing {
    pub same: bool,
    pub pairs: bool,
    pub tab: usize,
}

/// An edit around the caret: keep `keep` bytes of the text before it,
/// skip `skip` of the text after it, put `text` between, and leave the
/// caret `caret` bytes into `text`.
#[derive(Debug, PartialEq, Eq)]
pub struct Typed {
    pub keep: usize,
    pub skip: usize,
    pub text: String,
    pub caret: usize,
}

impl Typed {
    fn new(keep: usize, skip: usize, text: String) -> Typed {
        let caret = text.len();
        Typed { keep, skip, text, caret }
    }
}

/// Columns of leading whitespace as CodeMirror counts them, tabs `tab`
/// wide.
fn columns(s: &str, tab: usize) -> usize {
    let tab = tab.max(1);
    let mut n = 0;
    for c in s.chars() {
        match c {
            '\t' => n += tab - n % tab,
            c if c.is_whitespace() => n += 1,
            _ => break,
        }
    }
    n
}

/// What Enter types in Source Code Mode, as Typora's CodeMirror does
/// (its `newlineAndIndentContinueMarkdownList`, which leaves the numbers
/// of the items below alone). In a list item or a quote it starts the
/// next one; an empty item it clears, leaving a blank line, and an empty
/// quote it leaves; anywhere else the new line takes the indent of the
/// one before, in spaces. Between paired brackets it opens a line between
/// them. `before` is the line up to the caret (or the selection's start),
/// `after` the line from the caret (or the selection's end) on, and
/// `prev` the state the line before left.
pub fn enter(before: &str, after: &str, selected: bool, prev: State, t: Typing) -> Typed {
    static AT_START: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*([*+>-]|[0-9]+\.)\s").unwrap());
    let ws = |s: &str| s.len() - s.trim_start().len();
    let indent_of = |s: &str| " ".repeat(columns(s, t.tab));
    // Indented as the line it leaves, over what indent the rest had.
    let plain = || Typed::new(before.len(), ws(after), format!("\n{}", indent_of(before)));
    if t.pairs && !selected && matches!((before.chars().last(), after.chars().next()), (Some('['), Some(']')) | (Some('{'), Some('}'))) {
        let i = indent_of(before);
        return Typed { keep: before.len(), skip: 0, text: format!("\n{i}\n{i}"), caret: 1 + i.len() };
    }
    let line = format!("{before}{after}");
    let inside = matches!(prev.block, Block::Fence { .. } | Block::Math { .. } | Block::FrontMatter | Block::Html(_));
    if selected || inside || before.is_empty() && AT_START.is_match(&line) {
        return plain();
    }
    let Some(m) = ITEM.captures(&line) else { return plain() };
    // Only where Markdown reads a list item or a quote, not in code.
    let rest = line.trim_start_matches([' ', '\t']);
    let marked = indent(&line) < 4 || prev.list;
    let quote = marked && rest.starts_with('>');
    let item = marked && !parse::is_thematic_break(rest) && list_marker(rest).is_some_and(|w| w < rest.len());
    if !quote && !item {
        return plain();
    }
    if EMPTY_ITEM.is_match(&line) {
        if line.trim_end().ends_with('>') {
            return Typed::new(before.len(), 0, "\n".into());
        }
        let skip = after.chars().next().map_or(0, char::len_utf8);
        return Typed::new(0, skip, "\n".into());
    }
    let bullet = match (m.get(3), m.get(4)) {
        (Some(n), Some(d)) => format!("{}{}", n.as_str().parse::<u64>().unwrap_or(0) + u64::from(!t.same), d.as_str()),
        _ => m[2].replacen('x', " ", 1),
    };
    Typed::new(before.len(), 0, format!("\n{}{bullet}{}", &m[1], &m[5]))
}

/// Tab over a selection (`add`) and Shift+Tab (`!add`) in Source Code
/// Mode, as CodeMirror re-indents lines: Tab indents each a unit more,
/// Shift+Tab gives each the indent of the line above it (Markdown has no
/// smarter rule), in spaces either way. Lines of only spaces lose them,
/// but for the caret's line when nothing is `selected`. `above` is the
/// line before the first. The answer is each line's new text, and the
/// new columns of byte positions `marks` (line, column) in them.
pub fn reindent(lines: &[&str], above: Option<&str>, add: bool, selected: bool, tab: usize, marks: &[(usize, usize)]) -> (Vec<String>, Vec<usize>) {
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut moved = vec![(0, 0, false); lines.len()];
    for (n, line) in lines.iter().enumerate() {
        let old = line.len() - line.trim_start().len();
        let cols = if selected && line.trim().is_empty() {
            0
        } else if add {
            columns(line, tab) + tab
        } else {
            out.last().map(String::as_str).or(if n == 0 { above } else { None }).map_or(0, |l| columns(l, tab))
        };
        let new = " ".repeat(cols);
        moved[n] = (old, new.len(), new != line[..old]);
        out.push(format!("{new}{}", &line[old..]));
    }
    let cols = marks
        .iter()
        .map(|&(n, c)| {
            let (old, new, changed) = moved[n];
            // Positions in the old indent move to the end of the new one,
            // as CodeMirror maps them; an unchanged line's caret only
            // leaves its indent.
            if changed {
                if c <= old { new } else { c - old + new }
            } else if !selected && c < old {
                old
            } else {
                c
            }
        })
        .collect();
    (out, cols)
}

/// What typing a bracket or quote does in Source Code Mode, as
/// CodeMirror's closebrackets has it for Markdown.
#[derive(Debug, PartialEq, Eq)]
pub enum Pair {
    /// Type it as it is.
    Type,
    /// Step over the same character after the caret.
    Skip,
    /// Wrap the selection in the pair.
    Wrap(char, char),
    /// Type the pair, the caret between.
    Both(char, char),
}

const PAIRS: &str = "()[]{}''\"\"``";

/// Typing `ch` between `prev` and `next` (none at the line's ends).
pub fn pair(ch: char, prev: Option<char>, next: Option<char>, selected: bool) -> Pair {
    let Some(pos) = PAIRS.find(ch) else { return Pair::Type };
    let identical = PAIRS[pos + 1..].starts_with(ch);
    let opening = pos % 2 == 0;
    let (left, right) = if opening { (ch, PAIRS[pos + 1..].chars().next().unwrap()) } else { (PAIRS[pos - 1..].chars().next().unwrap(), ch) };
    let word = |c: char| c.is_alphanumeric() || c == '_';
    if opening && selected {
        Pair::Wrap(left, right)
    } else if (identical || !opening) && next == Some(ch) {
        Pair::Skip
    } else if identical {
        let prev = prev.unwrap_or(' ');
        if !next.is_some_and(word) && prev != ch && !word(prev) { Pair::Both(left, right) } else { Pair::Type }
    } else if opening && next.is_none_or(|c| c.is_whitespace() || ")]}'\":;>".contains(c)) {
        Pair::Both(left, right)
    } else {
        Pair::Type
    }
}

/// Whether Backspace between `prev` and `next` deletes a typed pair.
pub fn pair_around(prev: char, next: char) -> bool {
    let both: String = [prev, next].into_iter().collect();
    PAIRS.find(&both).is_some_and(|i| i % 2 == 0)
}

/// Container markers at the start of a line: quotes, and list items'
/// bullets and numbers. Where the line's own text starts, and whether an
/// item starts there.
fn markers(text: &str, out: &mut Layers) -> (usize, bool) {
    let mut at = 0;
    let mut listed = false;
    loop {
        let rest = &text[at..];
        let spaces = indent(rest);
        if spaces >= 4 {
            break;
        }
        let r = rest.trim_start_matches([' ', '\t']);
        let here = text.len() - r.len();
        if r.starts_with('>') {
            if !out.iter().any(|(_, l): &(Range<usize>, Look)| l.ink == Some(Ink::Quote)) {
                out.push((here..text.len(), Look::ink(Ink::Quote)));
            }
            at = here + 1 + usize::from(text[here + 1..].starts_with(' '));
            continue;
        }
        if parse::is_thematic_break(r) {
            at = here;
            break;
        }
        if let Some(width) = list_marker(r) {
            listed = true;
            at = here + width;
            at = text.len() - text[at..].trim_start_matches([' ', '\t']).len();
            continue;
        }
        at = here;
        break;
    }
    (at, listed)
}

/// The info string of a fence that `text` opens.
pub fn fence_info(text: &str) -> Option<String> {
    let (at, _) = markers(text, &mut Vec::new());
    parse::fence_start(&text[at..]).map(|(_, info)| info)
}

/// Where a line inside a fence starts, after the quote markers of the
/// quotes the fence is in.
pub fn fence_body(text: &str) -> usize {
    quotes(text, &mut Vec::new())
}

/// Columns of leading whitespace, a tab counting as four.
fn indent(s: &str) -> usize {
    let mut n = 0;
    for c in s.chars() {
        match c {
            ' ' => n += 1,
            '\t' => n += 4 - n % 4,
            _ => break,
        }
    }
    n
}

/// A bullet or number with a space after it (or nothing): its width.
fn list_marker(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let width = match b.first()? {
        b'-' | b'*' | b'+' => 1,
        b'0'..=b'9' => {
            let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
            if digits > 9 || !matches!(b.get(digits), Some(b'.' | b')')) {
                return None;
            }
            digits + 1
        }
        _ => return None,
    };
    matches!(b.get(width), None | Some(b' ' | b'\t')).then_some(width)
}

/// Quote markers at the start of a line inside a fence or math block, in
/// the quote colour. Returns where the content starts.
fn quotes(text: &str, out: &mut Layers) -> usize {
    let mut at = 0;
    loop {
        let r = text[at..].trim_start_matches(' ');
        if !r.starts_with('>') || indent(&text[at..]) >= 4 {
            return at;
        }
        let here = text.len() - r.len();
        out.push((here..here + 1, Look::ink(Ink::Quote)));
        at = here + 1 + usize::from(text[here + 1..].starts_with(' '));
    }
}

/// `[^label]:` at the start of a footnote: its length.
fn footnote_label(s: &str) -> Option<usize> {
    let close = s.strip_prefix("[^")?.find("]:")?;
    let label = &s[2..2 + close];
    (!label.is_empty() && !label.contains(char::is_whitespace)).then_some(close + 4)
}

/// A front matter line: YAML keys in the atom colour, comments as code.
fn front_matter(text: &str, out: &mut Layers) {
    let t = text.trim_start();
    let at = text.len() - t.len();
    if t.starts_with('#') {
        out.push((at..text.len(), Look::code()));
        return;
    }
    let key = t.strip_prefix("- ").map_or(t, |k| k);
    let start = text.len() - key.len();
    if let Some(colon) = key.find(':') {
        let name = &key[..colon];
        if !name.is_empty() && !name.contains(char::is_whitespace) && !name.starts_with(['"', '\'', '[', '{']) {
            out.push((start..start + colon, Look::ink(Ink::Atom)));
        }
    }
}

/// Inline markdown from `at` to the end of the line.
fn inlines(text: &str, at: usize, refs: &Refs, opts: &Options, out: &mut Layers) {
    let src = &text[at..];
    for n in inline::parse(src, refs, opts) {
        node(src, &n, at, out);
    }
}

fn node(src: &str, n: &Node, at: usize, out: &mut Layers) {
    let shift = |r: Range<usize>| r.start + at..r.end + at;
    let whole = shift(n.range.clone());
    match &n.kind {
        Kind::Code => out.push((whole, Look::code())),
        Kind::Math => out.push((whole, Look::ink(Ink::Math))),
        Kind::Emph => out.push((whole, Look { italic: true, ..Look::default() })),
        Kind::Strong => out.push((whole, Look { bold: true, ..Look::default() })),
        Kind::Strike => out.push((whole, Look { strike: true, ..Look::default() })),
        Kind::Highlight => out.push((whole, Look { mark: true, ..Look::default() })),
        // The bracketed text as a link, and what follows it (the address
        // and title, or the reference) as a string.
        Kind::Link { .. } | Kind::Image { .. } => {
            let head = n.range.start..(n.content.end + 1).min(n.range.end);
            out.push((shift(head.clone()), Look::link()));
            if head.end < n.range.end {
                out.push((shift(head.end..n.range.end), Look::ink(Ink::Url)));
            }
        }
        Kind::Autolink { .. } | Kind::FootnoteRef { .. } => out.push((whole, Look::link())),
        Kind::Comment => out.push((whole, Look::code())),
        Kind::HtmlTag { .. } | Kind::Html | Kind::ImgTag(_) => tags(src, n.range.clone(), at, out),
        // A pair of tags around formatted text.
        Kind::Tag { .. } => {
            tags(src, n.range.start..n.content.start, at, out);
            tags(src, n.content.end..n.range.end, at, out);
        }
        Kind::Escape | Kind::Entity(_) | Kind::Emoji(_) => out.push((whole, Look::ink(Ink::Atom))),
        _ => {}
    }
    for c in &n.children {
        node(src, c, at, out);
    }
}

/// HTML tags in `range` of `src`: the name and brackets in the tag colour,
/// quoted attribute values as strings, comments as code.
fn tags(src: &str, range: Range<usize>, at: usize, out: &mut Layers) {
    let b = src.as_bytes();
    let (mut i, end) = (range.start, range.end.min(b.len()));
    let mut push = |r: Range<usize>, look: Look| out.push((r.start + at..r.end + at, look));
    while i < end {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        if src[i..end].starts_with("<!--") {
            let close = src[i + 4..end].find("-->").map_or(end, |e| i + 4 + e + 3);
            push(i..close, Look::code());
            i = close;
            continue;
        }
        let start = i;
        i += 1;
        if i < end && matches!(b[i], b'/' | b'!' | b'?') {
            i += 1;
        }
        while i < end && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'-' | b':')) {
            i += 1;
        }
        if i == start + 1 {
            continue;
        }
        push(start..i, Look::ink(Ink::Tag));
        while i < end && b[i] != b'>' && b[i] != b'<' {
            if matches!(b[i], b'"' | b'\'') {
                let (q, from) = (b[i], i);
                i += 1;
                while i < end && b[i] != q {
                    i += 1;
                }
                i = (i + 1).min(end);
                push(from..i, Look::ink(Ink::Url));
            } else if b[i] == b'/' && b.get(i + 1) == Some(&b'>') && i + 1 < end {
                push(i..i + 1, Look::ink(Ink::Tag));
                i += 1;
            } else {
                i += 1;
            }
        }
        if i < end && b[i] == b'>' {
            push(i..i + 1, Look::ink(Ink::Tag));
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read lines in order and give each one's runs.
    fn read(src: &str) -> Vec<Vec<(Range<usize>, Look)>> {
        let (refs, opts) = (Refs::default(), Options::default());
        let mut state = State::default();
        let lines: Vec<&str> = src.split('\n').collect();
        lines
            .iter()
            .enumerate()
            .map(|(n, l)| {
                let after = lines.get(n + 1).copied().unwrap_or("");
                let (layers, next) = line(l, state, n == 0, after, &refs, &opts);
                assert_eq!(State::decode(next.encode()), next, "state of {l:?}");
                state = next;
                runs(l.len(), &layers)
            })
            .collect()
    }

    fn ink(i: Ink) -> Look {
        Look::ink(i)
    }

    #[test]
    fn headers() {
        let lines = read("# Title *x*\nText\n===\n### Three");
        assert_eq!(lines[0][0], (0..8, Look::header(1)));
        assert_eq!(lines[0][1], (8..11, Look { italic: true, ..Look::header(1) }));
        assert_eq!(Look::header(1).scale(), 2.0);
        // The text above an underline is a header too.
        assert_eq!(lines[1], vec![(0..4, Look::header(1))]);
        assert_eq!(lines[2], vec![(0..3, Look::header(1))]);
        assert_eq!(lines[3], vec![(0..9, Look::header(3))]);
        // Only the line just above it, as Typora's CodeMirror styles it.
        let lines = read("one\ntwo\n---\n- item\n===");
        assert!(lines[0].is_empty());
        assert_eq!(lines[1], vec![(0..3, Look::header(2))]);
        assert!(lines[3].iter().all(|(_, l)| l.level == 0));
    }

    #[test]
    fn fences_and_math_carry_over() {
        let lines = read("```rust\nlet x = *y*;\n```\n*a*\n$$\nx^2\n$$");
        assert_eq!(lines[0], vec![(0..7, Look::code())]);
        assert_eq!(lines[1], vec![(0..12, Look::code())]);
        assert_eq!(lines[2], vec![(0..3, Look::code())]);
        assert_eq!(lines[3], vec![(0..3, Look { italic: true, ..Look::default() })]);
        assert_eq!(lines[5], vec![(0..3, ink(Ink::Math).mono())]);
        let (_, after) = line("~~~~", State::default(), false, "", &Refs::default(), &Options::default());
        assert_eq!(after.block, Block::Fence { tilde: true, len: 4 });
        let (_, still) = line("~~~", after, false, "", &Refs::default(), &Options::default());
        assert_eq!(still, after);
    }

    #[test]
    fn front_matter_only_first() {
        let lines = read("---\ntitle: Notes\n---\n---");
        assert_eq!(lines[1], vec![(0..5, ink(Ink::Atom))]);
        assert_eq!(lines[2], vec![(0..3, ink(Ink::Rule))]);
        assert_eq!(lines[3], vec![(0..3, ink(Ink::Rule))]);
    }

    #[test]
    fn quotes_links_and_strings() {
        let lines = read("> a **b** [c](d.md \"t\") <i class='x'>");
        let quote = ink(Ink::Quote);
        assert_eq!(lines[0][0], (0..4, quote));
        assert_eq!(lines[0][1], (4..9, Look { bold: true, ..quote }));
        assert_eq!(lines[0][3], (10..13, Look::link()));
        assert_eq!(lines[0][4], (13..23, ink(Ink::Url)));
        assert_eq!(lines[0][6], (24..26, ink(Ink::Tag)));
        assert_eq!(lines[0][8], (33..36, ink(Ink::Url)));
        assert_eq!(lines[0][9], (36..37, ink(Ink::Tag)));
    }

    #[test]
    fn tables_rules_and_definitions() {
        let lines = read("| a | b |\n|---|---|\n| `c` | d |\n\n***\n[x]: http://e.com 'f'\n[^1]: Note");
        assert_eq!(lines[0], vec![(0..9, Look::default().mono())]);
        assert_eq!(lines[2][1], (2..5, Look::code()));
        assert_eq!(lines[4], vec![(0..3, ink(Ink::Rule))]);
        assert_eq!(lines[5], vec![(0..3, Look::link()), (5..17, ink(Ink::Url)), (18..21, ink(Ink::Url))]);
        assert_eq!(lines[6], vec![(0..5, Look::link())]);
    }

    #[test]
    fn html_blocks_and_comments() {
        let lines = read("<div id=\"a\">\n*not* em\n</div>\n\n<!-- one\ntwo -->\n*em*");
        assert_eq!(lines[0], vec![(0..4, ink(Ink::Tag)), (8..11, ink(Ink::Url)), (11..12, ink(Ink::Tag))]);
        assert!(lines[1].is_empty());
        assert_eq!(lines[4], vec![(0..8, Look::code())]);
        assert_eq!(lines[5], vec![(0..7, Look::code())]);
        assert_eq!(lines[6], vec![(0..4, Look { italic: true, ..Look::default() })]);
    }

    #[test]
    fn indented_code_outside_lists() {
        let lines = read("    code\n\n- item\n\n    more");
        assert_eq!(lines[0], vec![(0..8, Look::code())]);
        assert!(lines[4].is_empty());
    }

    #[test]
    fn states_round_trip() {
        for block in [
            Block::Blank,
            Block::Para,
            Block::Table,
            Block::FrontMatter,
            Block::Fence { tilde: true, len: 7 },
            Block::Math { brackets: true },
            Block::Html(5),
        ] {
            for list in [false, true] {
                let s = State { block, list, code: if matches!(block, Block::Fence { .. }) { 65535 } else { 0 } };
                assert_eq!(State::decode(s.encode()), s);
            }
        }
        assert_eq!(State::decode(-1), State::default());
    }

    const TYPING: Typing = Typing { same: false, pairs: true, tab: 4 };

    /// Enter at the `|` in `line`, as the line after `prev`.
    fn press(line: &str, prev: State) -> String {
        let (before, after) = line.split_once('|').unwrap();
        let t = enter(before, after, false, prev, TYPING);
        format!("{}{}|{}{}", &before[..t.keep], &t.text[..t.caret], &t.text[t.caret..], &after[t.skip..])
    }

    #[test]
    fn enter_continues_lists_and_quotes() {
        let top = State::default();
        let para = State { block: Block::Para, ..State::default() };
        assert_eq!(press("1. one|", top), "1. one\n2. |");
        assert_eq!(press("3) paren|", top), "3) paren\n4) |");
        assert_eq!(press("- [x] task|", top), "- [x] task\n- [ ] |");
        assert_eq!(press("* star|", top), "* star\n* |");
        assert_eq!(press("  - nested|", top), "  - nested\n  - |");
        assert_eq!(press("> quote|", top), "> quote\n> |");
        assert_eq!(press("> > deep|", top), "> > deep\n> > |");
        assert_eq!(press("- it|em", top), "- it\n- |em");
        // An empty item ends the list, an empty quote the quote.
        assert_eq!(press("- |", top), "\n|");
        assert_eq!(press("1. |", top), "\n|");
        assert_eq!(press("> |", top), "> \n|");
        // Elsewhere the new line keeps the indent.
        assert_eq!(press("plain|", top), "plain\n|");
        assert_eq!(press("    indented|", para), "    indented\n    |");
        assert_eq!(press("  ab|  cd", top), "  ab\n  |cd");
        assert_eq!(press("|- item", top), "\n|- item");
        assert_eq!(press("|1) item", top), "\n2) |1) item");
        assert_eq!(enter("1. ", "x", false, top, Typing { same: true, ..TYPING }).text, "\n1. ");
        assert_eq!(press("\tab|", top), "\tab\n    |");
        // Between brackets, a line between them.
        assert_eq!(press("  f{|}", top), "  f{\n  |\n  }");
        assert_eq!(press("  f(|)", top), "  f(\n  |)");
        assert_eq!(press("1.5 million|", top), "1.5 million\n|");
        assert_eq!(press("- - -|", top), "- - -\n|");
        // Code: indented, or in a fence.
        assert_eq!(press("    - code|", top), "    - code\n    |");
        assert_eq!(press("- in fence|", State { block: Block::Fence { tilde: false, len: 3 }, ..State::default() }), "- in fence\n|");
        assert_eq!(press("    - deeper|", State { block: Block::Para, list: true, code: 0 }), "    - deeper\n    - |");
    }

    #[test]
    fn tab_and_shift_tab_reindent() {
        let (lines, cols) = reindent(&["a", "\tb", "  "], None, true, true, 4, &[(0, 0), (2, 2)]);
        assert_eq!(lines, ["    a", "        b", ""]);
        assert_eq!(cols, [4, 0]);
        let (lines, cols) = reindent(&["x", "y"], Some("  top"), false, true, 4, &[(0, 1), (1, 1)]);
        assert_eq!(lines, ["  x", "  y"]);
        assert_eq!(cols, [3, 3]);
        // Shift+Tab with the caret in an indent that stays.
        let (lines, cols) = reindent(&["  z"], Some("  w"), false, false, 4, &[(0, 1)]);
        assert_eq!(lines, ["  z"]);
        assert_eq!(cols, [2]);
        assert_eq!(reindent(&["    "], None, false, false, 4, &[]).0, [""]);
    }

    #[test]
    fn brackets_pair() {
        assert_eq!(pair('(', Some('a'), None, false), Pair::Both('(', ')'));
        assert_eq!(pair('(', Some(' '), Some('x'), false), Pair::Type);
        assert_eq!(pair('[', None, Some(']'), false), Pair::Both('[', ']'));
        assert_eq!(pair(')', Some('a'), Some(')'), false), Pair::Skip);
        assert_eq!(pair(')', Some('a'), None, false), Pair::Type);
        assert_eq!(pair('"', Some(' '), None, false), Pair::Both('"', '"'));
        assert_eq!(pair('\'', Some('n'), Some('t'), false), Pair::Type);
        assert_eq!(pair('`', None, Some('`'), false), Pair::Skip);
        assert_eq!(pair('`', Some('`'), None, false), Pair::Type);
        assert_eq!(pair('*', None, None, false), Pair::Type);
        assert_eq!(pair('`', Some('a'), Some('b'), true), Pair::Wrap('`', '`'));
        assert_eq!(pair(']', None, None, true), Pair::Type);
        assert!(pair_around('(', ')') && pair_around('`', '`') && !pair_around(')', '('));
    }
}
