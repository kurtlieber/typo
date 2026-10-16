//! Auto-pairing as you type, following Typora's observed rules.
//!
//! In inline text (paragraphs, headings) brackets and quotes pair; with
//! "auto pair markdown syntax" on, `*`, `` ` `` and `_` do too (and `$` with
//! inline math). Code, math and HTML blocks follow CodeMirror's closebrackets
//! addon, which Typora uses for them. Positions are byte offsets into the
//! block's text.

use super::inline::{self, Kind, Node, Options, Refs};
use super::BlockKind;

/// What a typed character does at the caret.
#[derive(Debug, PartialEq, Eq)]
pub enum Typed {
    /// Insert it as usual.
    Insert,
    /// Step over the same character after the caret.
    Skip,
    /// Insert it and this closer after the caret.
    Pair(char),
}

fn closer(c: char) -> Option<char> {
    Some(match c {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '"' | '\'' | '*' | '`' | '_' | '$' | '~' | '^' | '=' => c,
        _ => return None,
    })
}

fn code_like(kind: &BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::CodeFence { .. } | BlockKind::IndentedCode | BlockKind::Math { .. } | BlockKind::Html | BlockKind::FrontMatter
    )
}

/// What typing `ch` at `at` in a block of `kind` with `text` does.
/// `markdown` is "auto pair markdown syntax".
pub fn typed(kind: &BlockKind, text: &str, at: usize, ch: char, markdown: bool, refs: &Refs, opts: &Options) -> Typed {
    if code_like(kind) { in_code(text, at, ch) } else { in_text(text, at, ch, markdown, refs, opts) }
}

/// Whether Backspace at `at` deletes an opener together with the closer
/// right after it.
pub fn paired(kind: &BlockKind, text: &str, at: usize, markdown: bool, opts: &Options) -> bool {
    let (Some(prev), Some(next)) = (text[..at].chars().next_back(), text[at..].chars().next()) else { return false };
    let opens = if code_like(kind) {
        "([{'\"".to_string()
    } else {
        let mut s = String::from("([{'\"");
        if markdown {
            s.push_str("*`~_");
            if opts.math {
                s.push('$');
            }
            if opts.superscript {
                s.push('^');
            }
        }
        s
    };
    opens.contains(prev) && closer(prev) == Some(next)
}

/// Letters as Typora's quote rule counts them: ASCII word characters,
/// Latin letters with accents, and Cyrillic.
fn wordish(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || c == '_'
        || (('\u{00C0}'..='\u{024F}').contains(&c) || ('\u{0400}'..='\u{04FF}').contains(&c)) && c.is_alphabetic()
}

fn in_text(text: &str, at: usize, ch: char, markdown: bool, refs: &Refs, opts: &Options) -> Typed {
    let before = &text[..at];
    let prev = before.chars().next_back();
    let next = text[at..].chars().next();
    match ch {
        ')' | ']' | '}' | '\'' | '"' => {
            if next == Some(ch) {
                return Typed::Skip;
            }
            if (ch == '\'' || ch == '"') && quote_pairs(text, before, prev, next, ch) {
                return Typed::Pair(ch);
            }
            Typed::Insert
        }
        // `\(` and `\[` open legacy math.
        '(' | '[' if prev == Some('\\') => Typed::Insert,
        '(' | '[' | '{' => match next {
            None => Typed::Pair(closer(ch).unwrap()),
            Some(n) if n.is_whitespace() || "[]{}()\"'`>$".contains(n) => Typed::Pair(closer(ch).unwrap()),
            _ => Typed::Insert,
        },
        '*' | '`' | '_' if markdown => symbol(text, at, ch, refs, opts),
        '$' if markdown && opts.math => symbol(text, at, ch, refs, opts),
        _ => Typed::Insert,
    }
}

fn quote_pairs(text: &str, before: &str, prev: Option<char>, next: Option<char>, ch: char) -> bool {
    if text.is_empty() {
        return true;
    }
    if prev == Some(ch) || prev == Some('\\') {
        return false;
    }
    // Not inside or right after a word ("don't"), nor after punctuation
    // that ends a phrase.
    if prev.is_some_and(wordish) || next.is_some_and(wordish) {
        return false;
    }
    if prev.is_some_and(|p| ".?,!:~)]".contains(p)) {
        return false;
    }
    !closes_quote(before, ch)
}

/// Whether `ch` would close a quote opened earlier in `before`. An
/// apostrophe inside a word opens nothing.
fn closes_quote(before: &str, ch: char) -> bool {
    let chars: Vec<char> = before.chars().collect();
    let mut open: Option<char> = None;
    let mut inner: Option<char> = None;
    for (i, &c) in chars.iter().enumerate() {
        let after_word = i > 0 && wordish(chars[i - 1]);
        let counts = c == '"' || (c == '\'' && (open.is_some() || !after_word));
        if !counts {
            continue;
        }
        match open {
            Some(o) if o == c => {
                open = None;
                inner = None;
            }
            Some(_) => inner = if inner.is_some() { None } else { Some(c) },
            None => open = Some(c),
        }
    }
    open == Some(ch) || inner == Some(ch)
}

fn symbol(text: &str, at: usize, ch: char, refs: &Refs, opts: &Options) -> Typed {
    let nodes = inline::parse(text, refs, opts);
    if ch != '`' && within(&nodes, at, &|n| n.kind == Kind::Code && n.content.start <= at && at <= n.content.end) {
        return Typed::Insert;
    }
    let before = &text[..at];
    let prev = before.chars().next_back();
    if opts.math && prev == Some('$') && ch != '$' {
        return Typed::Insert;
    }
    if prev == Some('\\') {
        return Typed::Insert;
    }
    let mut rest = text[at..].chars();
    let next = rest.next();
    let after = rest.next();
    if next == Some(ch) {
        // `*|*` and `**|**` grow into bold, except in the closing marker
        // of a span already made.
        let closing = within(&nodes, at, &|n| {
            matches!(n.kind, Kind::Strong | Kind::Emph) && n.content.end <= at && at < n.range.end
        });
        let grows = (ch == '*' || ch == '_') && after.is_none() && before.ends_with(['*', '_']) && !closing;
        return if grows { Typed::Pair(ch) } else { Typed::Skip };
    }
    let blank = prev.is_none_or(char::is_whitespace);
    if blank || !open_symbol(before, ch) {
        if blank && next.is_none_or(|n| n.is_whitespace() || ">)]}'\";:.,?!~*/&#=_+-".contains(n)) {
            return Typed::Pair(ch);
        }
        // After punctuation, as in `(*`.
        if prev.is_some_and(|p| !p.is_whitespace() && !p.is_alphanumeric() && p != '_') {
            return Typed::Pair(ch);
        }
    }
    Typed::Insert
}

/// Whether a `ch` span is open at the end of `before`: an odd count, with
/// `_` inside words not counted.
fn open_symbol(before: &str, ch: char) -> bool {
    if ch == '`' && before.ends_with("``") {
        return true;
    }
    let chars: Vec<char> = before.chars().collect();
    let mut open = false;
    for (i, &c) in chars.iter().enumerate() {
        if c != ch {
            continue;
        }
        let intraword = ch == '_'
            && i > 0
            && chars[i - 1].is_ascii_alphabetic()
            && chars.get(i + 1).is_some_and(char::is_ascii_alphabetic);
        if !intraword {
            open = !open;
        }
    }
    open
}

fn within(nodes: &[Node], at: usize, pred: &dyn Fn(&Node) -> bool) -> bool {
    nodes.iter().any(|n| n.range.start <= at && at <= n.range.end && (pred(n) || within(&n.children, at, pred)))
}

/// CodeMirror's closebrackets: pair `()[]{}''""`, skip over a closer, and
/// open a bracket only before space, the line end or a closing character.
fn in_code(text: &str, at: usize, ch: char) -> Typed {
    let next = text[at..].chars().next().filter(|&c| c != '\n');
    let prev = text[..at].chars().next_back().filter(|&c| c != '\n').unwrap_or(' ');
    let word = |c: char| c.is_alphanumeric() || c == '_';
    match ch {
        ')' | ']' | '}' if next == Some(ch) => Typed::Skip,
        '\'' | '"' => {
            if next == Some(ch) {
                Typed::Skip
            } else if !next.is_some_and(word) && prev != ch && !word(prev) {
                Typed::Pair(ch)
            } else {
                Typed::Insert
            }
        }
        '(' | '[' | '{' => match next {
            None => Typed::Pair(closer(ch).unwrap()),
            Some(n) if n.is_whitespace() || ")]}'\":;>".contains(n) => Typed::Pair(closer(ch).unwrap()),
            _ => Typed::Insert,
        },
        _ => Typed::Insert,
    }
}
