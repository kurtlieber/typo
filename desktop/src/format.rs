//! The Markdown toolbar's actions, as plain text edits. Each action looks at the text and the
//! selection, and gives back one replacement and the selection after it. The Android app has
//! the same rules.

/// A toolbar action.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fmt {
    Bold,
    Italic,
    Strike,
    Code,
    Link,
    Wiki,
    Heading,
    Bullet,
    Number,
    Task,
    Quote,
}

/// Replace `text[start..end]` with `insert`, then select `sel` (byte offsets in the new text).
#[derive(Debug, PartialEq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub insert: String,
    pub sel: (usize, usize),
}

pub fn apply(text: &str, a: usize, b: usize, f: Fmt) -> Edit {
    let (s, e) = (a.min(b).min(text.len()), a.max(b).min(text.len()));
    match f {
        Fmt::Bold => wrap(text, s, e, "**"),
        Fmt::Italic => wrap(text, s, e, "*"),
        Fmt::Strike => wrap(text, s, e, "~~"),
        Fmt::Code if text[s..e].contains('\n') => fence(text, s, e),
        Fmt::Code => wrap(text, s, e, "`"),
        Fmt::Link => link(text, s, e),
        Fmt::Wiki => wiki(text, s, e),
        Fmt::Heading | Fmt::Bullet | Fmt::Number | Fmt::Task | Fmt::Quote => lines(text, s, e, f),
    }
}

/// `m` is the whole mark at `at`: a `*` that is not part of `**`.
fn mark_at(text: &str, at: usize, m: &str) -> bool {
    if at + m.len() > text.len() || &text[at..at + m.len()] != m {
        return false;
    }
    if m != "*" {
        return true;
    }
    let before = at > 0 && text.as_bytes()[at - 1] == b'*';
    let after = text.as_bytes().get(at + 1) == Some(&b'*');
    !before && !after
}

/// Puts `m` on both sides of the selection, or takes it away when it is already there.
fn wrap(text: &str, s: usize, e: usize, m: &str) -> Edit {
    let n = m.len();
    // The marks sit just outside the selection.
    if s >= n && mark_at(text, s - n, m) && mark_at(text, e, m) {
        return Edit { start: s - n, end: e + n, insert: text[s..e].to_string(), sel: (s - n, e - n) };
    }
    // The selection holds the marks.
    let inner = &text[s..e];
    if inner.len() >= 2 * n && inner.starts_with(m) && inner.ends_with(m) && (m != "*" || (!inner.starts_with("**") && !inner.ends_with("**"))) {
        return Edit { start: s, end: e, insert: inner[n..inner.len() - n].to_string(), sel: (s, e - 2 * n) };
    }
    Edit { start: s, end: e, insert: format!("{m}{inner}{m}"), sel: (s + n, e + n) }
}

fn fence(text: &str, s: usize, e: usize) -> Edit {
    let lead = if s > 0 && text.as_bytes()[s - 1] != b'\n' { "\n" } else { "" };
    let body = text[s..e].trim_end_matches('\n');
    let insert = format!("{lead}```\n{body}\n```");
    let start_body = s + lead.len() + 4;
    Edit { start: s, end: e, sel: (start_body, start_body + body.len()), insert }
}

fn link(text: &str, s: usize, e: usize) -> Edit {
    let sel = &text[s..e];
    if sel.starts_with("http://") || sel.starts_with("https://") {
        return Edit { start: s, end: e, insert: format!("[]({sel})"), sel: (s + 1, s + 1) };
    }
    let insert = format!("[{sel}](url)");
    let u = s + 1 + sel.len() + 2;
    if sel.is_empty() {
        return Edit { start: s, end: e, insert, sel: (s + 1, s + 1) };
    }
    Edit { start: s, end: e, insert, sel: (u, u + 3) }
}

fn wiki(text: &str, s: usize, e: usize) -> Edit {
    if s >= 2 && text[..s].ends_with("[[") && text[e..].starts_with("]]") {
        return Edit { start: s - 2, end: e + 2, insert: text[s..e].to_string(), sel: (s - 2, e - 2) };
    }
    Edit { start: s, end: e, insert: format!("[[{}]]", &text[s..e]), sel: (s + 2, e + 2) }
}

/// The start of a line's text after a list marker, a task box or a quote mark.
fn prefix(line: &str) -> (&str, usize) {
    let t = line;
    for p in ["- [ ] ", "- [x] ", "- [X] ", "- ", "* ", "+ ", "> "] {
        if t.starts_with(p) {
            return (p, p.len());
        }
    }
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 && t[digits..].starts_with(". ") {
        return (&t[..digits + 2], digits + 2);
    }
    let hashes = t.bytes().take_while(|&c| c == b'#').count();
    if (1..=6).contains(&hashes) && t[hashes..].starts_with(' ') {
        return (&t[..hashes + 1], hashes + 1);
    }
    ("", 0)
}

fn kind(p: &str) -> Option<Fmt> {
    match p {
        "- [ ] " | "- [x] " | "- [X] " => Some(Fmt::Task),
        "- " | "* " | "+ " => Some(Fmt::Bullet),
        "> " => Some(Fmt::Quote),
        p if p.starts_with('#') => Some(Fmt::Heading),
        p if p.ends_with(". ") => Some(Fmt::Number),
        _ => None,
    }
}

/// The line's leading whitespace, in bytes.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Heading, lists, tasks and quotes work on whole lines: every line the selection touches. A
/// nested list item keeps its indent, and numbers count each level on its own.
fn lines(text: &str, s: usize, e: usize, f: Fmt) -> Edit {
    let ls = text[..s].rfind('\n').map_or(0, |i| i + 1);
    // A selection that ends at the start of a line leaves that line alone.
    let e_line = if e > s && text[..e].ends_with('\n') { e - 1 } else { e };
    let le = text[e_line..].find('\n').map_or(text.len(), |i| e_line + i);
    let block: Vec<&str> = text[ls..le].split('\n').collect();
    let many = block.len() > 1;
    let used: Vec<&&str> = block.iter().filter(|l| !many || !l.trim().is_empty()).collect();
    let all = !used.is_empty() && used.iter().all(|l| kind(prefix(l.trim_start()).0) == Some(f));
    let level = |l: &str| l.bytes().take_while(|&c| c == b'#').count();
    // The last number given at each indent. A line at one indent ends the deeper levels.
    let mut counts: Vec<(usize, u64)> = Vec::new();
    let out: Vec<String> = block
        .iter()
        .map(|l| {
            if many && l.trim().is_empty() {
                return l.to_string();
            }
            let (ind, body) = l.split_at(indent_of(l));
            let (p, len) = prefix(body);
            let rest = &body[len..];
            match f {
                Fmt::Heading => {
                    let next = if kind(p) == Some(Fmt::Heading) { (level(p) + 1) % 4 } else { 1 };
                    if next == 0 { rest.to_string() } else { format!("{} {rest}", "#".repeat(next)) }
                }
                _ if all => format!("{ind}{rest}"),
                Fmt::Bullet => format!("{ind}- {rest}"),
                Fmt::Task => format!("{ind}- [ ] {rest}"),
                Fmt::Quote => format!("> {l}"),
                Fmt::Number => {
                    counts.retain(|&(i, _)| i <= ind.len());
                    let n = match counts.last_mut() {
                        Some((i, n)) if *i == ind.len() => {
                            *n += 1;
                            *n
                        }
                        _ => {
                            counts.push((ind.len(), 1));
                            1
                        }
                    };
                    format!("{ind}{n}. {rest}")
                }
                _ => l.to_string(),
            }
        })
        .collect();
    let insert = out.join("\n");
    let sel = if many {
        (ls, ls + insert.len())
    } else {
        // One line: the caret keeps its place in the line's text.
        let shift = insert.len() as isize - (le - ls) as isize;
        let at = |x: usize| ((x as isize + shift).max(ls as isize) as usize).min(ls + insert.len());
        (at(s), at(e))
    };
    Edit { start: ls, end: le, insert, sel }
}

/// Tab (`out` false) and Shift+Tab (`out` true) on the selection.
///
/// On a list item, Tab nests the item under the item above it: it moves to that item's text
/// column, or to the column of that item's other children. Shift+Tab moves it out to its
/// parent's column. The lines under the item move with it, and a numbered item takes the
/// number its new place calls for. With several lines selected, every line moves by the same
/// step. Shift+Tab on another line takes up to two spaces off its start.
///
/// `None` leaves the key to the editor: Tab types two spaces where the caret is, in a line that
/// is no list item or inside a code block.
pub fn indent(text: &str, a: usize, b: usize, out: bool) -> Option<Edit> {
    let (s, e) = (a.min(b).min(text.len()), a.max(b).min(text.len()));
    let lines: Vec<&str> = text.split('\n').collect();
    let mut starts = Vec::with_capacity(lines.len());
    let mut at = 0;
    for l in &lines {
        starts.push(at);
        at += l.len() + 1;
    }
    let line_of = |pos: usize| starts.partition_point(|&st| st <= pos) - 1;
    // Lines in a fenced code block, fences included, are code, not list items.
    let mut code = Vec::with_capacity(lines.len());
    let mut fence = false;
    for l in &lines {
        let f = crate::md::is_fence(l);
        code.push(f || fence);
        fence ^= f;
    }
    let item = |i: usize| if code[i] { None } else { crate::md::list_line(lines[i]) };
    let first = line_of(s);
    let e_line = if e > s && text[..e].ends_with('\n') { e - 1 } else { e };
    let last = line_of(e_line);
    let noop = || Edit { start: s, end: s, insert: String::new(), sel: (a.min(text.len()), b.min(text.len())) };

    // The lines that move, the step, and a new number for one item.
    let (from, to, step, renumber): (usize, usize, usize, Option<(usize, u64)>) = if first == last {
        let i = first;
        if code[i] {
            return None;
        }
        let Some(l) = item(i) else {
            let n = lines[i].bytes().take_while(|&c| c == b' ').count().min(2);
            return (out && n > 0).then(|| shift(text, &lines, &starts, i, i, n, true, None, a, b));
        };
        let Some((step, number)) = (if out { outdent_step(&lines, &item, i, &l) } else { indent_step(&lines, &item, i, &l) }) else {
            return Some(noop());
        };
        (i, block_end(&lines, i, l.indent), step, number.map(|n| (i, n)))
    } else {
        let head = (first..=last).find_map(|i| item(i).map(|l| (i, l)));
        let step = match &head {
            Some((i, l)) => {
                let st = if out { outdent_step(&lines, &item, *i, l) } else { indent_step(&lines, &item, *i, l) };
                match st {
                    Some((st, _)) => st,
                    None => return Some(noop()),
                }
            }
            None => 2,
        };
        // An item at the end of the selection takes its children along.
        let to = (first..=last).filter_map(|i| item(i).map(|l| block_end(&lines, i, l.indent))).max().unwrap_or(last).max(last);
        (first, to, step, None)
    };
    Some(shift(text, &lines, &starts, from, to, step, out, renumber, a, b))
}

/// The step that nests item `i` under the item above it, and its new number if it has one.
/// `None` when no item above is at its level: a first item can't nest.
fn indent_step(lines: &[&str], item: &impl Fn(usize) -> Option<crate::md::ListLine>, i: usize, l: &crate::md::ListLine) -> Option<(usize, Option<u64>)> {
    let n = l.indent;
    // Up past the deeper lines to the item above at the same level. Its children, if any, give
    // the column: `kids` is the shallowest level seen, with the number of the nearest item there.
    let mut kids: Option<(usize, Option<u64>)> = None;
    let mut above = None;
    for j in (0..i).rev() {
        if lines[j].trim().is_empty() {
            continue;
        }
        let ind = indent_of(lines[j]);
        if ind > n {
            if let Some(k) = item(j)
                && kids.is_none_or(|(c, _)| ind < c)
            {
                kids = Some((ind, k.number.map(|(m, _)| m)));
            }
            continue;
        }
        if ind == n {
            above = item(j);
        }
        break;
    }
    let p = above?;
    // The text column: a run of five or more spaces after the marker counts as one.
    let col = if p.after - p.marker.end > 4 { p.marker.end + 1 } else { p.after };
    let target = match kids {
        Some((c, _)) if c <= col + 3 => c,
        _ => col,
    };
    let number = l.number.map(|_| match kids {
        Some((c, Some(m))) if c == target => m + 1,
        _ => 1,
    });
    Some((target - n, number))
}

/// The step that moves item `i` out to its parent's level, and its new number if it has one.
/// `None` for an item at the left edge.
fn outdent_step(lines: &[&str], item: &impl Fn(usize) -> Option<crate::md::ListLine>, i: usize, l: &crate::md::ListLine) -> Option<(usize, Option<u64>)> {
    let n = l.indent;
    if n == 0 {
        return None;
    }
    let mut parent = None;
    for j in (0..i).rev() {
        if lines[j].trim().is_empty() {
            continue;
        }
        if indent_of(lines[j]) < n {
            parent = item(j);
            break;
        }
    }
    let target = parent.as_ref().map_or(0, |p| p.indent);
    let number = match (&l.number, parent.as_ref().and_then(|p| p.number)) {
        (Some(_), Some((m, _))) => Some(m + 1),
        _ => None,
    };
    Some((n - target, number))
}

/// The last line of the item at `i`: the lines under it that sit deeper, without blank lines at
/// the end.
fn block_end(lines: &[&str], i: usize, indent: usize) -> usize {
    let mut end = i;
    for (j, l) in lines.iter().enumerate().skip(i + 1) {
        if l.trim().is_empty() {
            continue;
        }
        if indent_of(l) <= indent {
            break;
        }
        end = j;
    }
    end
}

/// Moves lines `from..=to` right (or left, `out`) by `step` columns. Blank lines stay as they
/// are, and a line moves left only as far as its spaces go. `renumber` gives an item line a new
/// number. The selection `a`, `b` keeps its place in the text.
#[allow(clippy::too_many_arguments)]
fn shift(text: &str, lines: &[&str], starts: &[usize], from: usize, to: usize, step: usize, out: bool, renumber: Option<(usize, u64)>, a: usize, b: usize) -> Edit {
    // Per line: the new text, and the old and new length of the part in front of its text.
    let mut new = Vec::new();
    for (k, l) in lines.iter().enumerate().take(to + 1).skip(from) {
        let ind = indent_of(l);
        let (mut pre_old, mut head, mut body) = (ind, String::new(), &l[ind..]);
        if l.trim().is_empty() {
            head.push_str(&l[..ind]);
        } else if out {
            let cut = l[..ind].bytes().take(step).take_while(|&c| c == b' ').count();
            head.push_str(&l[cut..ind]);
        } else {
            head = format!("{}{}", " ".repeat(step), &l[..ind]);
        }
        if let Some((_, num)) = renumber.filter(|&(r, _)| r == k)
            && let Some(li) = crate::md::list_line(l)
        {
            let (_, d) = li.number.unwrap_or((0, '.'));
            head.push_str(&format!("{num}{d}"));
            pre_old = li.marker.end;
            body = &l[li.marker.end..];
        }
        new.push((format!("{head}{body}"), pre_old, head.len()));
    }
    let start = starts[from];
    let end = starts[to] + lines[to].len();
    let insert = new.iter().map(|(t, _, _)| t.as_str()).collect::<Vec<_>>().join("\n");
    let grow = insert.len() as isize - (end - start) as isize;
    let map = |pos: usize| -> usize {
        if pos < start {
            return pos;
        }
        if pos > end {
            return (pos as isize + grow) as usize;
        }
        let mut at_old = start;
        let mut at_new = start;
        for (t, pre_old, pre_new) in &new {
            let len_old = t.len() + pre_old - pre_new;
            if pos <= at_old + len_old {
                let c = pos - at_old;
                let c2 = if c >= *pre_old { c - pre_old + pre_new } else { c.min(*pre_new) };
                return at_new + c2;
            }
            at_old += len_old + 1;
            at_new += t.len() + 1;
        }
        start + insert.len()
    };
    let sel = (map(a.min(text.len())), map(b.min(text.len())));
    Edit { start, end, insert, sel }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text after the edit, with `|` at the selection's ends.
    fn run(text: &str, f: Fmt) -> String {
        let a = text.find('|').unwrap();
        let t2 = text.replacen('|', "", 1);
        let b = t2.find('|').unwrap_or(a);
        let t = t2.replacen('|', "", 1);
        let ed = apply(&t, a, b, f);
        let mut out = format!("{}{}{}", &t[..ed.start], ed.insert, &t[ed.end..]);
        let (x, y) = ed.sel;
        out.insert(y, '|');
        if x != y {
            out.insert(x, '|');
        }
        out
    }

    #[test]
    fn marks_wrap_and_unwrap() {
        assert_eq!(run("a |word| b", Fmt::Bold), "a **|word|** b");
        assert_eq!(run("a **|word|** b", Fmt::Bold), "a |word| b");
        assert_eq!(run("a |**word**| b", Fmt::Bold), "a |word| b");
        assert_eq!(run("a | b", Fmt::Bold), "a **|** b");
        assert_eq!(run("a |word| b", Fmt::Italic), "a *|word|* b");
        assert_eq!(run("a *|word|* b", Fmt::Italic), "a |word| b");
        assert_eq!(run("**|word|**", Fmt::Italic), "***|word|***", "italic inside bold adds a mark");
        assert_eq!(run("|x|", Fmt::Strike), "~~|x|~~");
        assert_eq!(run("run |ls| now", Fmt::Code), "run `|ls|` now");
        assert_eq!(run("Text\n|a\nb|", Fmt::Code), "Text\n```\n|a\nb|\n```");
    }

    #[test]
    fn links() {
        assert_eq!(run("see |docs|", Fmt::Link), "see [docs](|url|)");
        assert_eq!(run("|https://x.io|", Fmt::Link), "[|](https://x.io)");
        assert_eq!(run("|", Fmt::Link), "[|](url)");
        assert_eq!(run("|Plan|", Fmt::Wiki), "[[|Plan|]]");
        assert_eq!(run("[[|Plan|]]", Fmt::Wiki), "|Plan|");
    }

    #[test]
    fn lines_toggle() {
        assert_eq!(run("Ti|tle", Fmt::Heading), "# Ti|tle");
        assert_eq!(run("# Ti|tle", Fmt::Heading), "## Ti|tle");
        assert_eq!(run("### Ti|tle", Fmt::Heading), "Ti|tle");
        assert_eq!(run("|a\nb|", Fmt::Bullet), "|- a\n- b|");
        assert_eq!(run("|- a\n- b|", Fmt::Bullet), "|a\nb|");
        assert_eq!(run("|a\n\nb|", Fmt::Number), "|1. a\n\n2. b|");
        assert_eq!(run("|- a\n- b|", Fmt::Number), "|1. a\n2. b|");
        assert_eq!(run("bu|y milk", Fmt::Task), "- [ ] bu|y milk");
        assert_eq!(run("- [ ] bu|y milk", Fmt::Task), "bu|y milk");
        assert_eq!(run("- bu|y", Fmt::Task), "- [ ] bu|y");
        assert_eq!(run("|said\nthis|", Fmt::Quote), "|> said\n> this|");
        assert_eq!(run("one\n|two\n|three", Fmt::Bullet), "one\n- |two|\nthree", "a selection that ends at a line start leaves that line");
    }

    #[test]
    fn nested_lines_keep_their_indent() {
        assert_eq!(run("- a\n  - |b|", Fmt::Bullet), "- a\n  |b|");
        assert_eq!(run("- a\n  b|", Fmt::Bullet), "- a\n  - b|");
        assert_eq!(run("- a\n  - [ ] b|", Fmt::Task), "- a\n  b|");
        assert_eq!(run("|a\n  b\n  c\nd|", Fmt::Number), "|1. a\n  1. b\n  2. c\n2. d|");
    }

    /// Tab (`out` false) or Shift+Tab at the `|` marks; `None` as "-" (the editor types spaces).
    fn tab(text: &str, out: bool) -> String {
        let a = text.find('|').unwrap();
        let t2 = text.replacen('|', "", 1);
        let b = t2.find('|').unwrap_or(a);
        let t = t2.replacen('|', "", 1);
        let Some(ed) = indent(&t, a, b, out) else {
            return "-".into();
        };
        let mut out = format!("{}{}{}", &t[..ed.start], ed.insert, &t[ed.end..]);
        let (x, y) = ed.sel;
        let (x, y) = (x.min(y), x.max(y));
        out.insert(y, '|');
        if x != y {
            out.insert(x, '|');
        }
        out
    }

    #[test]
    fn tab_nests_an_item() {
        assert_eq!(tab("- a\n- |b", false), "- a\n  - |b");
        assert_eq!(tab("- a\n- b|", false), "- a\n  - b|", "the caret can be anywhere on the line");
        assert_eq!(tab("- a\n|- b", false), "- a\n  |- b");
        assert_eq!(tab("1. a\n2. |b", false), "1. a\n   1. |b", "a numbered item goes under the text and starts at 1");
        assert_eq!(tab("- [ ] a\n- [ ] |b", false), "- [ ] a\n  - [ ] |b");
        assert_eq!(tab("- a\n  - b\n  - c\n- |d", false), "- a\n  - b\n  - c\n  - |d");
        assert_eq!(tab("- a\n  - b\n- |c", false), "- a\n  - b\n  - |c");
        assert_eq!(tab("- a\n    - b\n- |c", false), "- a\n    - b\n    - |c", "it joins the children where they are");
        assert_eq!(tab("1. a\n   1. b\n   2. c\n2. |d", false), "1. a\n   1. b\n   2. c\n   3. |d");
        assert_eq!(tab("- a\n  - b\n  - |c", false), "- a\n  - b\n    - |c", "a second level");
        assert_eq!(tab("- a\n\n- |b", false), "- a\n\n  - |b", "over a blank line");
    }

    #[test]
    fn tab_takes_the_children_along() {
        assert_eq!(tab("- a\n- |b\n  - c\n    - d\n- e", false), "- a\n  - |b\n    - c\n      - d\n- e");
        assert_eq!(tab("- a\n- |b\n  more b\n\n- e", false), "- a\n  - |b\n    more b\n\n- e");
    }

    #[test]
    fn tab_on_a_first_item_does_nothing() {
        assert_eq!(tab("- |a", false), "- |a");
        assert_eq!(tab("Text\n- |a", false), "Text\n- |a");
        assert_eq!(tab("- a\n  - |b", false), "- a\n  - |b");
    }

    #[test]
    fn tab_elsewhere_types_spaces() {
        assert_eq!(tab("plain |text", false), "-");
        assert_eq!(tab("```\n- |a\n```", false), "-", "inside a code block");
        assert_eq!(tab("plain |text", true), "-");
        assert_eq!(tab("  plain |text", true), "plain |text");
        assert_eq!(tab("    co|de", true), "  co|de");
    }

    #[test]
    fn shift_tab_moves_an_item_out() {
        assert_eq!(tab("- a\n  - |b", true), "- a\n- |b");
        assert_eq!(tab("- a\n  - b\n    - |c", true), "- a\n  - b\n  - |c");
        assert_eq!(tab("1. a\n   1. |b", true), "1. a\n2. |b", "a numbered item counts on from its parent");
        assert_eq!(tab("- a\n  - |b\n    - c\n- d", true), "- a\n- |b\n  - c\n- d", "with its children");
        assert_eq!(tab("- |a", true), "- |a");
        assert_eq!(tab("  - |a", true), "- |a", "an indented first item goes to the edge");
        assert_eq!(tab("- a\n    - |b", true), "- a\n- |b", "from any indent");
    }

    #[test]
    fn tab_moves_a_selection() {
        assert_eq!(tab("- a\n- |b\n- c|", false), "- a\n  - |b\n  - c|");
        assert_eq!(tab("- a\n  - |b\n  - c|", true), "- a\n- |b\n- c|");
        assert_eq!(tab("|one\ntwo|", false), "  |one\n  two|");
        assert_eq!(tab("  |one\n\n  two|", true), "|one\n\ntwo|");
        assert_eq!(tab("|- a\n- b|", false), "|- a\n- b|", "a first item can't nest, so nothing moves");
        assert_eq!(tab("- a\n- |b\n- c|\n  - d\n- e", false), "- a\n  - |b\n  - c|\n    - d\n- e");
    }
}
