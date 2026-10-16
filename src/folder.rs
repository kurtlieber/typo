//! The folder in the sidebar, as Typora shows it: the files it lists
//! (markdown and plain text, nothing hidden), as articles with a summary
//! or as a tree, in Typora's natural order; names for new files, folders
//! and duplicates; the folder-wide search; and Quick Open's ranking.
//! No Qt.

use std::cmp::Ordering;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::md;
use crate::md::query::FindOptions;

/// What Typora opens, so what it lists: its markdown extensions and the
/// other text formats it edits.
pub const EXTENSIONS: [&str; 15] = [
    "md", "markdown", "mmd", "mkd", "mdwn", "mdown", "mdx", "mdtxt", "mdtext", "apib", "rmarkdown", "rmd", "qmd", "txt", "text",
];

/// How many files the articles list shows before it gives up, as Typora's
/// does, pointing at the file tree instead.
pub const LIST_LIMIT: usize = 200;

/// How far the other walks go, so a home folder opened by mistake stays
/// usable.
const MAX_FILES: usize = 5000;
const MAX_DEPTH: usize = 12;

/// Whether the sidebar lists `path`.
pub fn is_listed(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase();
    EXTENSIONS.contains(&ext.as_str())
}

fn hidden(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'))
}

/// Which files the sidebar shows, from Preferences ▸ Files ▸ File Tree /
/// Library: hidden ones or not, and the files typo opens, every file, or
/// the files typo opens adjusted by glob patterns, one a line, "!" in
/// front to leave matches out. The last pattern that matches decides. A
/// pattern with a "/" in it matches the path under the open folder, any
/// other a name. Folders are shown unless a "!" pattern leaves them out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filter {
    pub hidden: bool,
    pub all: bool,
    pub patterns: Vec<String>,
}

impl Filter {
    /// From the preferences: `mode` is "supported", "all" or "custom".
    pub fn new(hidden: bool, mode: &str, patterns: &str) -> Filter {
        let patterns = if mode == "custom" {
            patterns.lines().map(str::trim).filter(|p| !p.is_empty() && !p.starts_with('#')).map(String::from).collect()
        } else {
            Vec::new()
        };
        Filter { hidden, all: mode == "all", patterns }
    }

    /// Whether the sidebar shows `path`, under the folder `root`.
    pub fn shows(&self, root: &Path, path: &Path, dir: bool) -> bool {
        if !self.hidden && hidden(path) {
            return false;
        }
        let mut shown = dir || self.all || is_listed(path);
        let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
        let name = name_of(path);
        for pattern in &self.patterns {
            let (exclude, pattern) = match pattern.strip_prefix('!') {
                Some(p) => (true, p.trim_start()),
                None => (false, pattern.as_str()),
            };
            // "name/" is a folder only.
            let (pattern, folder_only) = match pattern.strip_suffix('/') {
                Some(p) => (p, true),
                None => (pattern, false),
            };
            if (folder_only && !dir) || (dir && !exclude) {
                continue;
            }
            let pattern = pattern.strip_prefix("./").or_else(|| pattern.strip_prefix('/')).unwrap_or(pattern);
            let hit = if pattern.contains('/') { glob(pattern, &rel) } else { glob(pattern, &name) };
            if hit {
                shown = !exclude;
            }
        }
        shown
    }
}

/// Whether `text` matches the glob `pattern`: `*` and `?` within a part of
/// a path, `**` across parts (`**/` also matching no folder), `[…]` one of
/// a set of characters (`[!…]` none of them).
pub fn glob(pattern: &str, text: &str) -> bool {
    fn class(p: &[char], c: char) -> Option<(bool, usize)> {
        let negated = matches!(p.get(1), Some('!' | '^'));
        let start = if negated { 2 } else { 1 };
        let end = start + 1 + p.get(start + 1..)?.iter().position(|&x| x == ']')?;
        let set = &p[start..end];
        let mut hit = false;
        let mut i = 0;
        while i < set.len() {
            if i + 2 < set.len() && set[i + 1] == '-' {
                hit |= set[i] <= c && c <= set[i + 2];
                i += 3;
            } else {
                hit |= set[i] == c;
                i += 1;
            }
        }
        Some((hit != negated, end + 1))
    }
    fn go(p: &[char], t: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') if p.get(1) == Some(&'*') => {
                if p.get(2) == Some(&'/') {
                    (0..=t.len()).any(|i| (i == 0 || t[i - 1] == '/') && go(&p[3..], &t[i..]))
                } else {
                    (0..=t.len()).any(|i| go(&p[2..], &t[i..]))
                }
            }
            Some('*') => {
                for i in 0..=t.len() {
                    if go(&p[1..], &t[i..]) {
                        return true;
                    }
                    if t.get(i) == Some(&'/') {
                        break;
                    }
                }
                false
            }
            Some('?') => t.first().is_some_and(|&c| c != '/') && go(&p[1..], &t[1..]),
            Some('[') => match t.first().and_then(|&c| class(p, c).map(|r| (c, r))) {
                Some((c, (hit, len))) => hit && c != '/' && go(&p[len..], &t[1..]),
                None => t.first() == Some(&'[') && go(&p[1..], &t[1..]),
            },
            Some(c) => t.first() == Some(c) && go(&p[1..], &t[1..]),
        }
    }
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    go(&p, &t)
}

/// Milliseconds since the epoch, 0 when unknown.
fn millis(time: std::io::Result<std::time::SystemTime>) -> i64 {
    time.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as i64)
}

/// A file in the articles list.
#[derive(Clone, Debug, PartialEq)]
pub struct Article {
    pub path: String,
    /// The folder it is in.
    pub folder: String,
    pub modified: i64,
    pub created: i64,
    pub summary: String,
}

/// The order of the articles list: Typora's natural order, by name, by
/// modification date or by creation date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Natural,
    Name,
    Modified,
    Created,
}

impl Sort {
    pub fn parse(s: &str) -> Sort {
        match s {
            "name" => Sort::Name,
            "modified" => Sort::Modified,
            "created" => Sort::Created,
            _ => Sort::Natural,
        }
    }

    /// How two entries compare, given their names and dates; equal dates
    /// fall back to the natural order.
    fn cmp(self, (a, am, ac): (&str, i64, i64), (b, bm, bc): (&str, i64, i64)) -> Ordering {
        match self {
            Sort::Natural => natural_cmp(a, b),
            Sort::Name => name_cmp(a, b),
            Sort::Modified => am.cmp(&bm).then_with(|| natural_cmp(a, b)),
            Sort::Created => ac.cmp(&bc).then_with(|| natural_cmp(a, b)),
        }
    }
}

/// Typora's order by name: case is ignored, and names compare by what
/// comes before their first extension, then whole, characters as a locale
/// orders them (spaces and punctuation, then digits, then letters).
pub fn name_cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let base = |s: &str| -> String {
        let dot = s.char_indices().find(|&(i, c)| c == '.' && i + 1 < s.len()).map_or(s.len(), |(i, _)| i);
        s[..dot].to_string()
    };
    let rank = |c: char| (if c.is_alphabetic() { 2 } else if c.is_numeric() { 1 } else { 0 }, c);
    let collate = |x: &str, y: &str| x.chars().map(rank).cmp(y.chars().map(rank));
    collate(&base(&a), &base(&b)).then_with(|| collate(&a, &b))
}

/// Every listed file under `root`, folder by folder (the root's own files
/// first, then each subfolder's, depth first), sorted by `sort` within
/// each folder or, without `group`, all together. Summaries are read for
/// the first `summaries` of them. The flag says there were more than
/// `limit`, and the walk stopped there.
pub fn articles(
    root: &Path,
    filter: &Filter,
    sort: Sort,
    descending: bool,
    group: bool,
    summaries: usize,
    limit: usize,
) -> (Vec<Article>, bool) {
    let mut folders = Vec::new();
    let mut count = 0;
    let more = walk(root, root, filter, 0, &mut folders, &mut count, limit);
    let mut out = Vec::new();
    let order = |a: &Article, b: &Article| {
        let o = sort.cmp((&file_name(&a.path), a.modified, a.created), (&file_name(&b.path), b.modified, b.created));
        if descending { o.reverse() } else { o }
    };
    for (folder, files) in folders {
        let mut list: Vec<Article> = files
            .into_iter()
            .map(|p| {
                let meta = fs::metadata(&p).ok();
                Article {
                    path: p.to_string_lossy().into_owned(),
                    folder: folder.to_string_lossy().into_owned(),
                    modified: meta.as_ref().map_or(0, |m| millis(m.modified())),
                    created: meta.as_ref().map_or(0, |m| millis(m.created().or_else(|_| m.modified()))),
                    summary: String::new(),
                }
            })
            .collect();
        if group {
            list.sort_by(order);
        }
        out.extend(list);
    }
    if !group {
        out.sort_by(order);
    }
    for a in out.iter_mut().take(summaries) {
        a.summary = summary_of(Path::new(&a.path));
    }
    (out, more)
}

/// Collect `dir`'s listed files, then its subfolders' in natural order.
/// True when there were more than `limit`.
fn walk(
    root: &Path,
    dir: &Path,
    filter: &Filter,
    depth: usize,
    out: &mut Vec<(PathBuf, Vec<PathBuf>)>,
    count: &mut usize,
    limit: usize,
) -> bool {
    let Ok(entries) = fs::read_dir(dir) else { return false };
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for e in entries.flatten() {
        let path = e.path();
        // Symlinked folders are not followed, so a loop can't trap the walk.
        let Ok(kind) = e.file_type() else { continue };
        if kind.is_dir() {
            if filter.shows(root, &path, true) {
                dirs.push(path);
            }
        } else if filter.shows(root, &path, false) && (kind.is_file() || path.is_file()) {
            files.push(path);
        }
    }
    files.sort_by(|a, b| natural_cmp(&name_of(a), &name_of(b)));
    dirs.sort_by(|a, b| natural_cmp(&name_of(a), &name_of(b)));
    *count += files.len();
    if !files.is_empty() {
        out.push((dir.to_path_buf(), files));
    }
    if *count > limit {
        return true;
    }
    if depth < MAX_DEPTH {
        for d in dirs {
            if walk(root, &d, filter, depth + 1, out, count, limit) {
                return true;
            }
        }
    }
    false
}

fn name_of(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn file_name(path: &str) -> String {
    name_of(Path::new(path))
}

/// How a file or folder got where it was moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moved {
    Moved,
    /// Onto another disk, where it could only be copied; the original stays.
    Copied,
}

/// Move `from` to the path `to`, as Typora moves files dragged in the
/// tree: a rename, replacing a file already at `to` only with `replace`;
/// onto another disk, a copy.
pub fn move_path(from: &Path, to: &Path, replace: bool) -> Result<Moved, String> {
    if !replace && to.symlink_metadata().is_ok() {
        return Err("File/directory already exists at target path.".into());
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(Moved::Moved),
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
            copy_tree(from, to).map(|_| Moved::Copied).map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if fs::symlink_metadata(from)?.is_dir() {
        fs::create_dir_all(to)?;
        for e in fs::read_dir(from)? {
            let e = e?;
            copy_tree(&e.path(), &to.join(e.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(from, to).map(|_| ())
    }
}

/// Every listed file under `root`, in the articles list's natural order.
pub fn files(root: &Path, filter: &Filter) -> Vec<String> {
    let mut folders = Vec::new();
    walk(root, root, filter, 0, &mut folders, &mut 0, MAX_FILES);
    folders.into_iter().flat_map(|(_, f)| f).map(|p| p.to_string_lossy().into_owned()).collect()
}

/// An entry of the file tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub path: String,
    pub name: String,
    pub dir: bool,
}

/// What the tree shows inside `dir`, in the folder `root`, sorted by
/// `sort`: with `group`, its folders and then its listed files, each sorted
/// apart; without, all of them together.
pub fn children(root: &Path, dir: &Path, filter: &Filter, sort: Sort, descending: bool, group: bool) -> Vec<Node> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<(Node, i64, i64)> = entries
        .flatten()
        .map(|e| e.path())
        .filter_map(|p| {
            let dir = p.is_dir();
            filter.shows(root, &p, dir).then(|| {
                let meta = fs::metadata(&p).ok();
                let modified = meta.as_ref().map_or(0, |m| millis(m.modified()));
                let created = meta.as_ref().map_or(0, |m| millis(m.created().or_else(|_| m.modified())));
                (Node { name: name_of(&p), path: p.to_string_lossy().into_owned(), dir }, modified, created)
            })
        })
        .collect();
    out.sort_by(|(a, am, ac), (b, bm, bc)| {
        let o = sort.cmp((&a.name, *am, *ac), (&b.name, *bm, *bc));
        let o = if descending { o.reverse() } else { o };
        if group { b.dir.cmp(&a.dir).then(o) } else { o }
    });
    out.into_iter().map(|(n, _, _)| n).collect()
}

/// The start of a file as the articles list shows it: the text a reader
/// sees in its first blocks (headings without their `#`, inline markup
/// rendered away), a line each, about two lines' worth.
pub fn summary_of(path: &Path) -> String {
    let mut bytes = Vec::new();
    if let Ok(f) = fs::File::open(path) {
        let _ = f.take(4096).read_to_end(&mut bytes);
    }
    summary(&String::from_utf8_lossy(&bytes))
}

pub fn summary(text: &str) -> String {
    let doc = md::parse(text);
    let refs = md::inline::refs_of(&doc);
    let opts = md::inline::Options::default();
    let mut out: Vec<String> = Vec::new();
    let mut length = 0;
    for text in md::query::visible_text(&doc, &refs, &opts) {
        let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            continue;
        }
        length += line.chars().count();
        out.push(line);
        if length >= 160 || out.len() >= 3 {
            break;
        }
    }
    out.join("\n")
}

/// Typora's natural order: case is ignored; names are split into runs of
/// digits, separators and other text; numbers compare by value and come
/// after words, month names compare by month, and a name that is a prefix
/// of another comes first.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (ta, tb) = (tokens(&a), tokens(&b));
    for (x, y) in ta.iter().zip(&tb) {
        if x == y {
            continue;
        }
        let (dx, dy) = (x.starts_with(|c: char| c.is_ascii_digit()), y.starts_with(|c: char| c.is_ascii_digit()));
        let o = match (dx, dy) {
            (true, true) => compare_digits(x, y),
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => match (month(x), month(y)) {
                (Some(m), Some(n)) => m.cmp(&n),
                _ => x.cmp(y),
            },
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    ta.len().cmp(&tb.len())
}

/// Numbers of any length, by value.
fn compare_digits(x: &str, y: &str) -> Ordering {
    let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
    x.len().cmp(&y.len()).then_with(|| x.cmp(y))
}

fn tokens(s: &str) -> Vec<&str> {
    let kind = |c: char| {
        if c.is_ascii_digit() {
            0
        } else if matches!(c, '.' | ',' | '+' | '-') || c.is_whitespace() {
            1
        } else {
            2
        }
    };
    let mut out = Vec::new();
    let mut start = 0;
    let mut last = None;
    for (i, c) in s.char_indices() {
        let k = kind(c);
        if last.is_some_and(|l| l != k) {
            out.push(&s[start..i]);
            start = i;
        }
        last = Some(k);
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

fn month(s: &str) -> Option<usize> {
    const MONTHS: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    const FULL: [&str; 12] = [
        "january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december",
    ];
    // As Typora's pattern has it, April only in full.
    MONTHS.iter().zip(FULL).position(|(m, f)| (s == *m && s != "apr") || s == f)
}

/// The name New File gives in `dir`: `Untitled.md`, then `Untitled 1.md`
/// and up, one past the highest taken, as Typora numbers them; New Folder
/// likewise gives `Untitled Folder`, `Untitled Folder 1`…
pub fn untitled_name(dir: &Path, folder: bool, ext: &str) -> String {
    let base = if folder { "untitled folder" } else { "untitled" };
    let mut next: Option<u64> = None;
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_lowercase();
        let is_dir = e.path().is_dir();
        if is_dir != folder {
            continue;
        }
        let stem = if folder { name.as_str() } else { name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s) };
        let Some(rest) = stem.strip_prefix(base) else { continue };
        let n = if rest.is_empty() { Some(0) } else { rest.strip_prefix(' ').and_then(|d| d.parse::<u64>().ok()) };
        if let Some(n) = n {
            next = Some(next.map_or(n + 1, |m| m.max(n + 1)));
        }
    }
    let label = if folder { "Untitled Folder" } else { "Untitled" };
    let name = match next {
        None | Some(0) => label.to_string(),
        Some(n) => format!("{label} {n}"),
    };
    if folder { name } else { format!("{name}.{ext}") }
}

/// Where Duplicate copies `path` to: `name (copy).ext`, with another
/// ` (copy)` for each that is already taken.
pub fn duplicate_path(path: &Path) -> PathBuf {
    let dir = path.parent().unwrap_or(Path::new("."));
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let mut name = stem;
    loop {
        name = format!("{name} (copy)");
        let candidate = dir.join(format!("{name}{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
}

/// A file the folder search found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    /// Matches in the file's text.
    pub count: usize,
    pub lines: Vec<HitLine>,
}

/// A line with matches: its number from 0, the text shown (trimmed to
/// keep long lines short) and the matches in it as UTF-16 ranges of that
/// text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HitLine {
    pub line: usize,
    pub text: String,
    pub ranges: Vec<(usize, usize)>,
}

/// Search every listed file under `root` for `query`: files whose text
/// matches, most matches first, then those whose name alone matches. The
/// error is the regular expression's.
pub fn search(root: &Path, filter: &Filter, query: &str, opts: &FindOptions) -> Result<Vec<Hit>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let re = md::query::matcher(query, opts).map_err(|e| e.to_string())?;
    let mut hits = Vec::new();
    let mut named = Vec::new();
    for path in files(root, filter) {
        let Ok(bytes) = fs::read(&path) else { continue };
        // Big files and binary ones (a NUL early on) are only searched by
        // name.
        if bytes.len() > 4 << 20 || bytes[..bytes.len().min(8000)].contains(&0) {
            if re.is_match(&file_name(&path)) {
                named.push(Hit { path, count: 0, lines: Vec::new() });
            }
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        let mut count = 0;
        let mut lines = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let found: Vec<_> = re.find_iter(line).filter(|m| !m.is_empty()).map(|m| m.range()).collect();
            if found.is_empty() {
                continue;
            }
            count += found.len();
            if lines.len() < 50 {
                lines.push(excerpt(n, line, &found));
            }
        }
        if count > 0 {
            hits.push(Hit { path, count, lines });
        } else if re.is_match(&file_name(&path)) {
            named.push(Hit { path, count: 0, lines: Vec::new() });
        }
    }
    hits.sort_by(|a, b| b.count.cmp(&a.count));
    hits.extend(named);
    Ok(hits)
}

/// A line cut down to the matches with some text around them.
fn excerpt(line: usize, text: &str, found: &[std::ops::Range<usize>]) -> HitLine {
    const BEFORE: usize = 30;
    const LENGTH: usize = 160;
    let first = found[0].start;
    let mut start = first.saturating_sub(BEFORE);
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    // Start at a word when the cut is inside one.
    if start > 0
        && let Some(space) = text[start..first].find(char::is_whitespace)
    {
        start += space + 1;
    }
    let mut end = (start + LENGTH).min(text.len()).max(found[0].end);
    while !text.is_char_boundary(end) {
        end += 1;
    }
    let shown = &text[start..end];
    let prefix = if start > 0 { "…" } else { "" };
    let suffix = if end < text.len() { "…" } else { "" };
    let utf16 = |byte: usize| prefix.encode_utf16().count() + shown[..byte].encode_utf16().count();
    let ranges = found
        .iter()
        .filter(|r| r.start >= start && r.end <= end)
        .map(|r| (utf16(r.start - start), utf16(r.end - start)))
        .collect();
    HitLine { line, text: format!("{prefix}{}{suffix}", shown.trim_end()), ranges }
}

/// Quick Open: the `candidates` (paths) whose names match `query`, best
/// first. Every word of the query must be in the path, in any case; a
/// name that starts with the query ranks above one that contains it,
/// which ranks above a match in the folders alone. With an empty query,
/// the candidates as they are.
pub fn quick_open(candidates: &[String], query: &str, root: &str) -> Vec<String> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return candidates.to_vec();
    }
    let joined = words.join(" ");
    let mut scored: Vec<(u32, usize, &String)> = candidates
        .iter()
        .enumerate()
        .filter_map(|(i, path)| {
            let rel = path.strip_prefix(root).unwrap_or(path).to_lowercase();
            let name = file_name(path).to_lowercase();
            if !words.iter().all(|w| rel.contains(w.as_str())) {
                return None;
            }
            let score = if name.starts_with(&joined) {
                0
            } else if name.contains(&joined) {
                1
            } else if words.iter().all(|w| name.contains(w.as_str())) {
                2
            } else {
                3
            };
            Some((score, i, path))
        })
        .collect();
    scored.sort();
    scored.into_iter().map(|(_, _, p)| p.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> tempfile_lite::Dir {
        let d = tempfile_lite::Dir::new();
        d.write("welcome.md", "# Welcome\n\nThis is *the* welcome note.\n\n## Section\n\nBody.\n");
        d.write("ideas.md", "# Ideas\n\nGarden and kitchen.\n");
        d.write("notes.txt", "plain text\n");
        d.write("image.png", "x");
        d.write(".secret.md", "hidden");
        d.write(".hidden/a.md", "hidden");
        d.write("journal/2026-10-12.md", "# Monday\n\nWent to the garden.\n");
        d.write("projects/README.md", "Readme, no heading.\n");
        d.write("projects/alpha/plan.md", "# Plan\n\nThe garden plan.\n");
        d
    }

    #[test]
    fn lists_articles_by_folder() {
        let d = tree();
        let (list, more) = articles(d.path(), &Filter::default(), Sort::Natural, false, true, 100, LIST_LIMIT);
        assert!(!more);
        let names: Vec<String> = list.iter().map(|a| a.path.strip_prefix(d.path().to_str().unwrap()).unwrap().to_string()).collect();
        assert_eq!(names, ["/ideas.md", "/notes.txt", "/welcome.md", "/journal/2026-10-12.md", "/projects/README.md", "/projects/alpha/plan.md"]);
        assert_eq!(list[2].summary, "Welcome\nThis is the welcome note.\nSection");
        assert_eq!(list[1].folder, d.path().to_str().unwrap());
        // Without groups, one order across folders.
        let (list, _) = articles(d.path(), &Filter::default(), Sort::Name, true, false, 0, LIST_LIMIT);
        assert_eq!(file_name(&list[0].path), "welcome.md");
        assert_eq!(list[0].summary, "");
        // Past the limit, the list gives up.
        assert!(articles(d.path(), &Filter::default(), Sort::Natural, false, true, 0, 5).1);
        assert!(!articles(d.path(), &Filter::default(), Sort::Natural, false, true, 0, 6).1);
    }

    #[test]
    fn tree_children() {
        let d = tree();
        let kids = |sort, descending, group| -> Vec<String> {
            children(d.path(), d.path(), &Filter::default(), sort, descending, group).into_iter().map(|n| n.name).collect()
        };
        assert_eq!(kids(Sort::Natural, false, true), ["journal", "projects", "ideas.md", "notes.txt", "welcome.md"]);
        assert_eq!(kids(Sort::Natural, true, true), ["projects", "journal", "welcome.md", "notes.txt", "ideas.md"]);
        assert_eq!(kids(Sort::Name, false, false), ["ideas.md", "journal", "notes.txt", "projects", "welcome.md"]);
        // By date, oldest first.
        let at = |name: &str, secs: u64| {
            let t = UNIX_EPOCH + std::time::Duration::from_secs(secs);
            fs::File::options().write(true).open(d.path().join(name)).unwrap().set_modified(t).unwrap();
        };
        at("welcome.md", 3000);
        at("ideas.md", 2000);
        at("notes.txt", 1000);
        assert_eq!(kids(Sort::Modified, false, true)[2..], ["notes.txt", "ideas.md", "welcome.md"]);
        assert_eq!(kids(Sort::Modified, true, true)[2..], ["welcome.md", "ideas.md", "notes.txt"]);
        // The folders were made just now, so come after them.
        assert_eq!(kids(Sort::Modified, false, false)[..3], ["notes.txt", "ideas.md", "welcome.md"]);
    }

    #[test]
    fn moves() {
        let d = tree();
        let p = |s: &str| d.path().join(s);
        assert_eq!(move_path(&p("ideas.md"), &p("journal/ideas.md"), false), Ok(Moved::Moved));
        assert!(!p("ideas.md").exists() && p("journal/ideas.md").exists());
        assert_eq!(move_path(&p("projects"), &p("journal/projects"), false), Ok(Moved::Moved));
        assert!(p("journal/projects/alpha/plan.md").exists());
        // A name already there stays, unless replaced.
        assert!(move_path(&p("welcome.md"), &p("journal/ideas.md"), false).is_err());
        assert!(p("welcome.md").exists());
        assert_eq!(move_path(&p("welcome.md"), &p("journal/ideas.md"), true), Ok(Moved::Moved));
        assert!(fs::read_to_string(p("journal/ideas.md")).unwrap().starts_with("# Welcome"));
        // Copying, as onto another disk.
        copy_tree(&p("journal"), &p("copy")).unwrap();
        assert!(p("copy/projects/alpha/plan.md").exists() && p("journal/projects/alpha/plan.md").exists());
    }

    #[test]
    fn name_order() {
        let mut names = vec!["b.md", "a b.md", "A.txt", "a.md", "a1.md", "a-z.md", "file10.md", "file2.md", ".x"];
        names.sort_by(|a, b| name_cmp(a, b));
        assert_eq!(names, [".x", "a.md", "A.txt", "a b.md", "a-z.md", "a1.md", "b.md", "file10.md", "file2.md"]);
    }

    #[test]
    fn filters() {
        let d = tree();
        let rel = |list: Vec<String>| -> Vec<String> {
            list.iter().map(|p| p.strip_prefix(d.path().to_str().unwrap()).unwrap().to_string()).collect()
        };
        let all = Filter::new(true, "all", "*.png");
        assert_eq!(all.patterns, Vec::<String>::new());
        let got = rel(files(d.path(), &all));
        assert!(got.contains(&"/image.png".into()) && got.contains(&"/.secret.md".into()) && got.contains(&"/.hidden/a.md".into()));
        let custom = Filter::new(false, "custom", "*.png\n!**/alpha/\n!journal/*.md\n# a comment");
        assert_eq!(
            rel(files(d.path(), &custom)),
            ["/ideas.md", "/image.png", "/notes.txt", "/welcome.md", "/projects/README.md"]
        );
        let kids: Vec<String> =
            children(d.path(), &d.path().join("projects"), &custom, Sort::Natural, false, true).into_iter().map(|n| n.name).collect();
        assert_eq!(kids, ["README.md"]);
        assert!(glob("**/draft-*.md", "draft-1.md") && glob("**/draft-*.md", "a/b/draft-1.md"));
        assert!(!glob("**/draft-*.md", "a/xdraft-1.md") && !glob("*.md", "a/b.md"));
        assert!(glob("a/**", "a/b/c") && glob("file?.[mt]d", "file1.md") && !glob("file?.[!mt]d", "file1.md"));
        assert!(glob("[abc", "[abc"));
    }

    #[test]
    fn natural_order() {
        let mut names = vec!["file10.md", "File2.md", "file1.md", "notes", "2026-10-01", "Notes 2", "a.md", "a b.md"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(names, ["a b.md", "a.md", "file1.md", "File2.md", "file10.md", "notes", "Notes 2", "2026-10-01"]);
        // Month names by month, against each other.
        let mut months = vec!["report march", "report Feb", "report december", "report april"];
        months.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(months, ["report Feb", "report march", "report april", "report december"]);
    }

    #[test]
    fn new_names() {
        let d = tempfile_lite::Dir::new();
        assert_eq!(untitled_name(d.path(), false, "md"), "Untitled.md");
        d.write("Untitled.md", "");
        assert_eq!(untitled_name(d.path(), false, "md"), "Untitled 1.md");
        d.write("untitled 4.md", "");
        assert_eq!(untitled_name(d.path(), false, "md"), "Untitled 5.md");
        assert_eq!(untitled_name(d.path(), true, "md"), "Untitled Folder");
        d.write("Untitled Folder/x.md", "");
        assert_eq!(untitled_name(d.path(), true, "md"), "Untitled Folder 1");
        d.write("a.md", "");
        assert_eq!(duplicate_path(&d.path().join("a.md")), d.path().join("a (copy).md"));
        d.write("a (copy).md", "");
        assert_eq!(duplicate_path(&d.path().join("a.md")), d.path().join("a (copy) (copy).md"));
    }

    #[test]
    fn searches_the_folder() {
        let d = tree();
        let opts = FindOptions::default();
        let hits = search(d.path(), &Filter::default(), "garden", &opts).unwrap();
        let names: Vec<String> = hits.iter().map(|h| file_name(&h.path)).collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"plan.md".to_string()) && names.contains(&"ideas.md".to_string()));
        let plan = hits.iter().find(|h| h.path.ends_with("plan.md")).unwrap();
        assert_eq!(plan.lines, [HitLine { line: 2, text: "The garden plan.".into(), ranges: vec![(4, 10)] }]);
        // A name alone matches after the texts.
        let hits = search(d.path(), &Filter::default(), "readme", &opts).unwrap();
        assert_eq!((file_name(&hits[0].path).as_str(), hits[0].count), ("README.md", 1));
        let hits = search(d.path(), &Filter::default(), "ideas", &FindOptions { case_sensitive: true, ..opts }).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].count, 0);
        assert!(search(d.path(), &Filter::default(), "(", &FindOptions { regex: true, ..opts }).is_err());
        // Long lines are cut around the match.
        let long = format!("{} needle {}", "word ".repeat(40), "tail ".repeat(60));
        let line = excerpt(0, &long, &[long.find("needle").unwrap()..long.find("needle").unwrap() + 6]);
        assert!(line.text.starts_with("…word") && line.text.ends_with('…'));
        let (a, b) = line.ranges[0];
        assert_eq!(line.text.chars().skip(a).take(b - a).collect::<String>(), "needle");
    }

    #[test]
    fn quick_open_ranks() {
        let paths: Vec<String> =
            ["/r/notes/plan.md", "/r/plan/old.md", "/r/my plan.md", "/r/planning.md"].iter().map(|s| s.to_string()).collect();
        assert_eq!(quick_open(&paths, "plan", "/r"), ["/r/notes/plan.md", "/r/planning.md", "/r/my plan.md", "/r/plan/old.md"]);
        assert_eq!(quick_open(&paths, "notes plan", "/r"), ["/r/notes/plan.md"]);
        assert_eq!(quick_open(&paths, "", "/r"), paths);
    }

    /// A throwaway directory for the tests.
    mod tempfile_lite {
        use std::path::{Path, PathBuf};

        pub struct Dir(PathBuf);

        impl Dir {
            pub fn new() -> Dir {
                use std::sync::atomic::{AtomicUsize, Ordering};
                static N: AtomicUsize = AtomicUsize::new(0);
                let p = std::env::temp_dir().join(format!("typo-folder-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
                std::fs::create_dir_all(&p).unwrap();
                Dir(p)
            }
            pub fn path(&self) -> &Path {
                &self.0
            }
            pub fn write(&self, rel: &str, text: &str) {
                let p = self.0.join(rel);
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(p, text).unwrap();
            }
        }

        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
}
