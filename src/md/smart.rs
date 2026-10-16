//! Smart punctuation (Preferences ▸ Markdown ▸ Smart Punctuation), as Typora
//! does it: curly quotes, dashes and ellipses, either typed in place of the
//! ASCII ones (convert on input) or shown for them while the file keeps
//! ASCII (convert on rendering). No Qt.

use std::ops::Range;

use super::inline::{Kind, Node};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Smart {
    pub quotes: bool,
    pub dashes: bool,
    /// `--` makes an em dash too, not an en dash.
    pub to_em: bool,
    /// The opening and closing double quotes, then the single ones.
    pub pairs: [char; 4],
}

impl Default for Smart {
    fn default() -> Self {
        Smart { quotes: false, dashes: false, to_em: false, pairs: ['“', '”', '‘', '’'] }
    }
}

impl Smart {
    /// `pairs` as Preferences keeps them, four characters ("“”‘’").
    pub fn new(quotes: bool, dashes: bool, to_em: bool, pairs: &str) -> Smart {
        let chars: Vec<char> = pairs.chars().collect();
        let pairs = <[char; 4]>::try_from(chars).unwrap_or(Smart::default().pairs);
        Smart { quotes, dashes, to_em, pairs }
    }

    pub fn any(&self) -> bool {
        self.quotes || self.dashes
    }

    fn dash(&self) -> char {
        if self.to_em { '—' } else { '–' }
    }

    /// Whether a quote typed after `before` opens, as Typora decides: at
    /// the start, after a bracket, space, dash, slash or opening single
    /// quote, after a markdown marker that follows a space, or inside the
    /// other kind of quote.
    fn opens(&self, before: &str, single: bool) -> bool {
        let mut rev = before.chars().rev();
        let Some(last) = rev.next() else { return true };
        if "({[-/‘".contains(last) || last.is_whitespace() {
            return true;
        }
        if "*`~+^$_".contains(last) && rev.next().is_none_or(char::is_whitespace) {
            return true;
        }
        if single { last == self.pairs[0] } else { last == self.pairs[2] }
    }
}

/// What typing `ch` does with smart punctuation converted on input: the
/// characters before the caret (at byte `at` in a block's `text`, parsed
/// as `nodes`) it replaces, and what goes in their place and the typed
/// character's. None to type it as it is: in code, math, a URL or raw HTML,
/// after a backslash, or where nothing converts.
pub fn typed(text: &str, nodes: &[Node], at: usize, ch: char, smart: &Smart) -> Option<(usize, String)> {
    if !plain_at(nodes, at) {
        return None;
    }
    // Escaped punctuation counts for nothing.
    let before = escaped().replace_all(&text[..at], "");
    if before.ends_with('\\') {
        return None;
    }
    match ch {
        '-' if smart.dashes => {
            // A line of dashes stays one: a rule, a setext underline or a
            // list marker in the making.
            let line = &text[text[..at].rfind('\n').map_or(0, |i| i + 1)..at];
            if line.chars().all(|c| c == '-') {
                None
            } else if before.ends_with("--") {
                Some((2, "—".into()))
            } else if before.ends_with('–') {
                Some((1, "—".into()))
            } else if before.ends_with('-') {
                Some((1, smart.dash().into()))
            } else {
                None
            }
        }
        '.' if smart.dashes && before.ends_with("..") => Some((2, "…".into())),
        '\'' if smart.quotes => Some((0, smart.pairs[if smart.opens(&before, true) { 2 } else { 3 }].into())),
        '"' if smart.quotes => Some((0, smart.pairs[if smart.opens(&before, false) { 0 } else { 1 }].into())),
        _ => None,
    }
}

fn escaped() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r#"\\[-."']"#).unwrap())
}

/// Whether byte `at` is in plain text: not inside code, math, raw HTML, an
/// autolink or a link's destination.
fn plain_at(nodes: &[Node], at: usize) -> bool {
    nodes.iter().all(|n| {
        if !(n.range.start < at && at < n.range.end) {
            return true;
        }
        match &n.kind {
            Kind::Code | Kind::Math | Kind::Autolink { .. } | Kind::HtmlTag { .. } | Kind::Html | Kind::Comment | Kind::Escape => false,
            Kind::Link { .. } | Kind::Image { .. } if at > n.content.end => false,
            _ => plain_at(&n.children, at),
        }
    })
}

/// Convert on rendering: the ASCII punctuation in the plain text of a
/// block's `text`, parsed as `nodes`, that shows smart, and what as. As
/// Typora renders it: `---` an em dash, `--` an en dash (or em), `...` an
/// ellipsis, and quotes curled by what is around them.
pub fn rendered(text: &str, nodes: &[Node], smart: &Smart) -> Vec<(Range<usize>, char)> {
    let mut runs = Vec::new();
    text_runs(nodes, &mut runs);
    let mut out: Vec<(Range<usize>, char)> = Vec::new();
    // Whether the last double quote opened; the apostrophe that may yet
    // turn out to open a single quote.
    let mut double_open = false;
    let mut open_single: Option<usize> = None;
    for run in runs {
        let mut i = run.start;
        while i < run.end {
            let rest = &text[i..run.end];
            let next_after = |n: usize| text[i + n..].chars().next();
            if smart.dashes {
                if rest.starts_with("---") && next_after(3) != Some('-') {
                    out.push((i..i + 3, '—'));
                    i += 3;
                    continue;
                }
                if rest.starts_with("--") {
                    out.push((i..i + 2, smart.dash()));
                    i += 2;
                    continue;
                }
                if rest.starts_with("...") && next_after(3) != Some('.') {
                    out.push((i..i + 3, '…'));
                    i += 3;
                    continue;
                }
            }
            let before = &text[..i];
            if smart.quotes && rest.starts_with('"') {
                let last = before.chars().next_back();
                let opens = last.is_some_and(|c| "({[-/‘".contains(c) || c.is_whitespace()) || last == Some(smart.pairs[2]);
                let closes = next_after(1).is_none_or(|c| ")/.,;:?!’".contains(c) || c.is_whitespace());
                let open = opens || !closes && !double_open;
                double_open = open;
                out.push((i..i + 1, smart.pairs[if open { 0 } else { 1 }]));
                i += 1;
                continue;
            }
            if smart.quotes && rest.starts_with('\'') {
                let closes = next_after(1).is_none_or(|c| ")\".,;:?!-".contains(c) || c.is_whitespace());
                if let Some(o) = open_single.filter(|_| closes) {
                    out[o].1 = smart.pairs[2];
                    open_single = None;
                    out.push((i..i + 1, smart.pairs[3]));
                } else if rest.len() == 1 && before.chars().next_back().is_none_or(char::is_whitespace) {
                    // A lone quote at the end after a space stays.
                } else {
                    // An apostrophe, unless a closing quote comes.
                    let apostrophe = if smart.pairs[3] == '’' { '’' } else { '\'' };
                    if before.chars().next_back().is_none_or(|c| "(\"-".contains(c) || c.is_whitespace()) {
                        open_single = Some(out.len());
                    }
                    out.push((i..i + 1, apostrophe));
                }
                i += 1;
                continue;
            }
            i += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    out.retain(|(r, c)| text[r.clone()].chars().ne(std::iter::once(*c)));
    out
}

/// The ranges of plain text in `nodes`, in order.
fn text_runs(nodes: &[Node], out: &mut Vec<Range<usize>>) {
    for n in nodes {
        match n.kind {
            Kind::Text => out.push(n.range.clone()),
            Kind::Code | Kind::Math | Kind::Autolink { .. } | Kind::Image { .. } => {}
            _ => text_runs(&n.children, out),
        }
    }
}

/// `nodes` with the punctuation `rendered` converts split out of their
/// text as `Kind::Smart` nodes.
pub fn split(text: &str, nodes: Vec<Node>, smart: &Smart) -> Vec<Node> {
    let marks = rendered(text, &nodes, smart);
    if marks.is_empty() {
        return nodes;
    }
    fn walk(nodes: Vec<Node>, marks: &[(Range<usize>, char)]) -> Vec<Node> {
        let mut out = Vec::with_capacity(nodes.len());
        for mut n in nodes {
            if n.kind != Kind::Text {
                n.children = walk(std::mem::take(&mut n.children), marks);
                out.push(n);
                continue;
            }
            let mut at = n.range.start;
            for (r, c) in marks.iter().filter(|(r, _)| n.range.start <= r.start && r.end <= n.range.end) {
                if r.start > at {
                    out.push(Node { kind: Kind::Text, range: at..r.start, content: at..r.start, children: Vec::new() });
                }
                out.push(Node { kind: Kind::Smart(*c), range: r.clone(), content: r.clone(), children: Vec::new() });
                at = r.end;
            }
            if at < n.range.end {
                out.push(Node { kind: Kind::Text, range: at..n.range.end, content: at..n.range.end, children: Vec::new() });
            }
        }
        out
    }
    walk(nodes, &marks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::inline::{self, Options, Refs};

    fn smart() -> Smart {
        Smart { quotes: true, dashes: true, ..Smart::default() }
    }

    fn typing(text: &str, ch: char) -> Option<(usize, String)> {
        let nodes = inline::parse(text, &Refs::new(), &Options::default());
        typed(text, &nodes, text.len(), ch, &smart())
    }

    #[test]
    fn converts_as_it_is_typed() {
        assert_eq!(typing("a", '-'), None);
        assert_eq!(typing("a -", '-'), Some((1, "–".into())));
        assert_eq!(typing("a –", '-'), Some((1, "—".into())));
        assert_eq!(typing("wait..", '.'), Some((2, "…".into())));
        assert_eq!(typing("", '"'), Some((0, "“".into())));
        assert_eq!(typing("say ", '"'), Some((0, "“".into())));
        assert_eq!(typing("say “hi", '"'), Some((0, "”".into())));
        assert_eq!(typing("don", '\''), Some((0, "’".into())));
        assert_eq!(typing("“", '\''), Some((0, "‘".into())));
        assert_eq!(typing("*", '"'), Some((0, "“".into())));
        // Not in code, after a backslash, or on a line of dashes.
        assert_eq!(typing("`a", '"'), Some((0, "”".into())));
        assert_eq!(typing("`a` b", '"'), Some((0, "”".into())));
        let nodes = inline::parse("`ab`", &Refs::new(), &Options::default());
        assert_eq!(typed("`ab`", &nodes, 2, '"', &smart()), None);
        assert_eq!(typing("a\\", '"'), None);
        assert_eq!(typing("--", '-'), None);
        assert_eq!(typing("x\n-", '-'), None);
        // The pairs and the em dash are the preferences'.
        let german = Smart { to_em: true, ..Smart::new(true, true, false, "„“‚‘") };
        let nodes = inline::parse("a -", &Refs::new(), &Options::default());
        assert_eq!(typed("a -", &nodes, 3, '-', &german), Some((1, "—".into())));
        assert_eq!(typed("", &[], 0, '"', &german), Some((0, "„".into())));
    }

    fn shown(text: &str) -> String {
        let nodes = inline::parse(text, &Refs::new(), &Options::default());
        let mut out = text.to_string();
        for (r, c) in rendered(text, &nodes, &smart()).into_iter().rev() {
            out.replace_range(r, &c.to_string());
        }
        out
    }

    #[test]
    fn converts_as_it_is_rendered() {
        assert_eq!(shown(r#"He said "it's 'fine'" -- or --- not..."#), "He said “it’s ‘fine’” – or — not…");
        assert_eq!(shown(r#""Quoted" at the start"#), "“Quoted” at the start");
        assert_eq!(shown("`a--b` and a--b"), "`a--b` and a–b");
        assert_eq!(shown("the '90s"), "the ’90s");
    }
}
