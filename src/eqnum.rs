//! Equation numbers, as MathJax gives them under Typora's "Auto Numbering
//! for Math Equations" preference. `\tag{…}` always shows. Under the AMS
//! rules the numbering environments (`equation`, `align`, `gather`…) number
//! themselves, a number for each row of those with rows; numbering all
//! equations gives every other display formula one number too. `\notag` or
//! `\nonumber` leaves a row out, and `\label{…}` names a row's number for
//! `\eqref{…}` and `\ref{…}` anywhere in the document.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

use regex::{Captures, Regex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Numbering {
    None,
    Ams,
    All,
}

impl Numbering {
    /// Typora's setting: "", "ams" or "all".
    pub fn from_setting(s: &str) -> Self {
        match s {
            "ams" => Numbering::Ams,
            "all" => Numbering::All,
            _ => Numbering::None,
        }
    }
}

/// The numbering environments, and whether each has rows of its own.
const NUMBERED: [(&str, bool); 7] = [
    ("equation", false),
    ("multline", false),
    ("align", true),
    ("flalign", true),
    ("alignat", true),
    ("gather", true),
    ("eqnarray", true),
];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Tags {
    /// For each formula, what each of its rows shows at the right, top to
    /// bottom: `(1)`, a `\tag` as written in brackets, or "" for nothing.
    pub rows: Vec<Vec<String>>,
    /// What each label refers to: a number, or a tag's text.
    pub labels: HashMap<String, String>,
}

/// Number display formulas `formulas`, in document order.
pub fn number(formulas: &[&str], numbering: Numbering) -> Tags {
    let mut tags = Tags::default();
    let mut count = 0;
    for tex in formulas {
        let (rows, auto) = rows(tex, numbering);
        let mut shown = Vec::new();
        for row in rows {
            let tag = find(row, "tag").into_iter().find_map(|(star, arg)| Some((star, arg?)));
            let skip = !find(row, "notag").is_empty() || !find(row, "nonumber").is_empty();
            let (text, refers) = match tag {
                Some((true, t)) => (t.to_string(), t.to_string()),
                Some((false, t)) => (format!("({t})"), t.to_string()),
                None if auto && !skip => {
                    count += 1;
                    (format!("({count})"), count.to_string())
                }
                None => (String::new(), String::new()),
            };
            for (_, label) in find(row, "label") {
                if let Some(label) = label.filter(|_| !refers.is_empty()) {
                    tags.labels.entry(label.trim().to_string()).or_insert_with(|| refers.clone());
                }
            }
            shown.push(text);
        }
        tags.rows.push(shown);
    }
    tags
}

/// `tex` with what its rows show, `shown`, written into it for a converter
/// that numbers nothing itself: a numbering environment starred, each row
/// without a `\tag` given its number as one, and a formula outside such an
/// environment put in `equation*` when it has a tag or a label, which only
/// numbering environments may hold.
pub fn tagged(tex: &str, shown: &[String]) -> String {
    let tex = tex.trim();
    let tag = |row: &str, i: usize| match shown.get(i) {
        Some(t) if !t.is_empty() && find(row, "tag").is_empty() => format!(" \\tag*{{{t}}}"),
        _ => String::new(),
    };
    if let Some((env, _, body)) = outer_environment(tex)
        && let Some(&(_, has_rows)) = NUMBERED.iter().find(|(e, _)| *e == env)
    {
        let rows = if has_rows { split_rows(body) } else { vec![body] };
        let mut out = format!("\\begin{{{env}*}}");
        let mut at = 0;
        for (i, row) in rows.iter().enumerate() {
            let end = row.as_ptr() as usize - body.as_ptr() as usize + row.len();
            out.push_str(&body[at..end]);
            out.push_str(&tag(row, i));
            at = end;
        }
        out.push_str(&body[at..]);
        out.push_str(&format!("\\end{{{env}*}}"));
        return out;
    }
    let tag = tag(tex, 0);
    if tag.is_empty() && find(tex, "tag").is_empty() && find(tex, "label").is_empty() {
        return tex.to_string();
    }
    format!("\\begin{{equation*}}{tex}{tag}\\end{{equation*}}")
}

static REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\\(eqref|ref)\s*\{([^{}]*)\}").unwrap());

/// `tex` with each `\eqref{…}` and `\ref{…}` written out as what its label
/// refers to (`labels`, as `Tags` has them), `???` for a label no formula
/// has, as MathJax shows them.
pub fn with_refs<'a>(tex: &'a str, labels: &HashMap<String, String>) -> Cow<'a, str> {
    REF.replace_all(tex, |c: &Captures| {
        let n = labels.get(c[2].trim()).map_or("???", String::as_str);
        if &c[1] == "eqref" { format!("\\text{{({n})}}") } else { format!("\\text{{{n}}}") }
    })
}

/// Where each row of a formula of several rows begins, as byte offsets
/// into `tex`; empty for one row.
pub fn row_starts(tex: &str) -> Vec<usize> {
    match rows(tex, Numbering::All).0 {
        rows if rows.len() > 1 => rows.iter().map(|r| r.as_ptr() as usize - tex.as_ptr() as usize).collect(),
        _ => Vec::new(),
    }
}

/// The formula's rows, and whether they number themselves.
fn rows(tex: &str, numbering: Numbering) -> (Vec<&str>, bool) {
    let tex = tex.trim();
    if tex.is_empty() {
        return (Vec::new(), false);
    }
    if let Some((env, star, body)) = outer_environment(tex)
        && let Some(&(_, has_rows)) = NUMBERED.iter().find(|(e, _)| *e == env)
    {
        let auto = !star && numbering != Numbering::None;
        let mut rows = if has_rows { split_rows(body) } else { vec![body] };
        // A last `\\` ends the last row rather than starting another.
        if rows.len() > 1 && rows.last().is_some_and(|r| r.trim().is_empty()) {
            rows.pop();
        }
        return (rows, auto);
    }
    (vec![tex], numbering == Numbering::All)
}

/// `(name, starred, body)` when the whole of `tex` is one environment.
fn outer_environment(tex: &str) -> Option<(&str, bool, &str)> {
    let after = tex.strip_prefix("\\begin")?;
    let (name, body_start) = braced(after.trim_start())?;
    let mut depth = 0;
    let mut i = 0;
    let bytes = tex.as_bytes();
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }
        let rest = &tex[i..];
        if rest.starts_with("\\begin") && !word_continues(&rest[6..]) {
            depth += 1;
        } else if rest.starts_with("\\end") && !word_continues(&rest[4..]) {
            depth -= 1;
            if depth == 0 {
                let (_, tail) = braced(rest[4..].trim_start())?;
                if !tail.trim().is_empty() {
                    return None;
                }
                let body = &tex[tex.len() - body_start.len()..i];
                let (env, star) = match name.strip_suffix('*') {
                    Some(env) => (env, true),
                    None => (name, false),
                };
                return Some((env.trim(), star, body));
            }
        }
        i += 2;
    }
    None
}

/// The rows of an environment's body: its parts between top-level `\\`.
fn split_rows(body: &str) -> Vec<&str> {
    let bytes = body.as_bytes();
    let (mut braces, mut envs) = (0i32, 0i32);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => braces += 1,
            b'}' => braces -= 1,
            b'\\' => {
                let rest = &body[i..];
                if rest.starts_with("\\\\") {
                    if braces == 0 && envs == 0 {
                        rows.push(&body[start..i]);
                        start = i + 2;
                    }
                } else if rest.starts_with("\\begin") && !word_continues(&rest[6..]) {
                    envs += 1;
                } else if rest.starts_with("\\end") && !word_continues(&rest[4..]) {
                    envs -= 1;
                }
                i += 2;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    rows.push(&body[start..]);
    rows
}

/// Whether a command's name goes on: `\beginx` is not `\begin`.
fn word_continues(rest: &str) -> bool {
    rest.starts_with(|c: char| c.is_ascii_alphabetic())
}

/// The first `\name` in `s` of `names`: the name and what follows it.
fn command_at<'a, 'n>(s: &'a str, names: &[&'n str]) -> Option<(&'n str, &'a str)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            let rest = &s[i + 1..];
            if let Some(name) = names.iter().find(|n| rest.starts_with(**n) && !word_continues(&rest[n.len()..])) {
                return Some((name, &rest[name.len()..]));
            }
            // Skip what the backslash escapes, `\\` included.
            i += 2;
            continue;
        }
        i += 1;
    }
    None
}

/// Each `\name` in `tex`: whether it is starred, and its argument.
fn find<'a>(tex: &'a str, name: &str) -> Vec<(bool, Option<&'a str>)> {
    let mut out = Vec::new();
    let mut rest = tex;
    while let Some((_, after)) = command_at(rest, &[name]) {
        let (star, after) = match after.strip_prefix('*') {
            Some(a) => (true, a),
            None => (false, after),
        };
        match braced(after.trim_start()) {
            Some((arg, tail)) => {
                out.push((star, Some(arg)));
                rest = tail;
            }
            None => {
                out.push((star, None));
                rest = after;
            }
        }
    }
    out
}

/// `{…}` at the start of `s`: what is inside, and what follows.
fn braced(s: &str) -> Option<(&str, &str)> {
    let inner = s.strip_prefix('{')?;
    let mut depth = 1;
    let mut escaped = false;
    for (i, c) in inner.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&inner[..i], &inner[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_of(formulas: &[&str], numbering: Numbering) -> Vec<Vec<String>> {
        number(formulas, numbering).rows
    }

    #[test]
    fn numbers_as_mathjax_does() {
        let doc = [
            "x",
            r"\begin{equation} y \label{one} \end{equation}",
            "\\begin{align}\n a &= b \\label{two} \\\\\n c &= d \\notag \\\\\n e &= f \\tag{T} \\label{t}\n\\end{align}",
            r"\begin{align*} g \\ h \end{align*}",
            r"\begin{aligned} i \\ j \end{aligned}",
            r"k \tag*{star}",
            "",
        ];
        assert_eq!(
            rows_of(&doc, Numbering::Ams),
            [vec![""], vec!["(1)"], vec!["(2)", "", "(T)"], vec!["", ""], vec![""], vec!["star"], vec![]]
        );
        assert_eq!(
            rows_of(&doc, Numbering::All),
            [vec!["(1)"], vec!["(2)"], vec!["(3)", "", "(T)"], vec!["", ""], vec!["(4)"], vec!["star"], vec![]]
        );
        // Tags show whatever the setting.
        assert_eq!(rows_of(&doc, Numbering::None)[2], ["", "", "(T)"]);
        let labels = number(&doc, Numbering::Ams).labels;
        assert_eq!((labels["one"].as_str(), labels["two"].as_str(), labels["t"].as_str()), ("1", "2", "T"));
    }

    #[test]
    fn rows_end_at_top_level_breaks() {
        let tex = "\\begin{gather} \\begin{pmatrix} 1 \\\\ 2 \\end{pmatrix} \\\\ {a \\\\ b} \\\\ \\end{gather}";
        assert_eq!(rows_of(&[tex], Numbering::Ams), [vec!["(1)", "(2)"]]);
        // Not one environment: one formula, one number.
        let two = r"\begin{align} a \end{align} \begin{align} b \end{align}";
        assert_eq!(rows_of(&[two], Numbering::Ams), [vec![""]]);
        assert_eq!(rows_of(&[r"\begin{alignat}{2} a &= b \\ c &= d \end{alignat}"], Numbering::Ams), [vec!["(1)", "(2)"]]);
    }

    #[test]
    fn finds_where_rows_begin() {
        let tex = r" \begin{align} a \\ b \\ \end{align}";
        let starts = row_starts(tex);
        assert_eq!(starts.iter().map(|&at| &tex[at..at + 2]).collect::<Vec<_>>(), [" a", " b"]);
        assert!(row_starts(r"\begin{align} a \end{align}").is_empty() && row_starts(r"a \\ b").is_empty());
    }

    #[test]
    fn writes_out_references() {
        let labels = HashMap::from([("a".to_string(), "2".to_string())]);
        assert_eq!(with_refs(r"x = \eqref{a} + \ref{ a } - \eqref{b}", &labels), r"x = \text{(2)} + \text{2} - \text{(???)}");
        assert!(matches!(with_refs("x", &labels), Cow::Borrowed(_)));
    }

    #[test]
    fn writes_numbers_in_as_tags() {
        let shown = |rows: &[&str]| rows.iter().map(|r| r.to_string()).collect::<Vec<_>>();
        assert_eq!(tagged(" x ", &shown(&["(1)"])), r"\begin{equation*}x \tag*{(1)}\end{equation*}");
        assert_eq!(tagged("x", &shown(&[""])), "x");
        assert_eq!(tagged(r"x \label{a}", &[]), r"\begin{equation*}x \label{a}\end{equation*}");
        assert_eq!(
            tagged(r"\begin{align} a \\ b \tag{B} \\ c \end{align}", &shown(&["(2)", "(B)", ""])),
            r"\begin{align*} a  \tag*{(2)}\\ b \tag{B} \\ c \end{align*}"
        );
        assert_eq!(tagged(r"\begin{alignat}{2} a \end{alignat}", &shown(&["(1)"])), r"\begin{alignat*}{2} a  \tag*{(1)}\end{alignat*}");
    }
}
