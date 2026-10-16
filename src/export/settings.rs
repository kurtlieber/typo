//! Preferences ▸ Export as Typora applies them: a format's options with
//! what the document's front matter says over some of them, and the
//! `${…}` variables in headers, footers, appended HTML and commands.
//! No Qt.

use std::path::Path;
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;
use serde_json::{Map, Value};
use yaml_rust2::{Yaml, YamlLoader};

use crate::md::{BlockKind, Document};

/// The front matter keys every export reads.
const ALWAYS: &[&str] = &["title", "author", "subject", "creator", "split-level", "toc-depth"];
/// The ones it reads only when the format's "Read and overwrite export
/// settings from YAML front matters" is on: a file from someone else
/// can't otherwise put markup in the page.
const PER_FILE: &[&str] = &["header", "footer", "sidebar", "append-head", "append-head-extra", "append-body", "append-body-extra"];

static VARIABLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\{([^\r\n}]+)\}").unwrap());
static META: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?is)<meta\b[^>]*>"#).unwrap());
static CONTENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?is)(\bcontent\s*=\s*)("[^"]*"|'[^']*')"#).unwrap());
static TITLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)(<title\b[^>]*>)(.*?)(</title>)").unwrap());

/// The document's front matter as a JSON object; empty when there is
/// none, or it isn't a mapping.
pub fn front_matter(doc: &Document) -> Value {
    let yaml = doc
        .blocks
        .first()
        .filter(|b| b.kind == BlockKind::FrontMatter)
        .and_then(|b| YamlLoader::load_from_str(&b.text).ok())
        .and_then(|docs| docs.into_iter().next());
    match yaml.map(json) {
        Some(v @ Value::Object(_)) => v,
        _ => Value::Object(Map::new()),
    }
}

fn json(y: Yaml) -> Value {
    match y {
        Yaml::Real(s) => s.parse::<f64>().ok().and_then(serde_json::Number::from_f64).map_or(Value::String(s), Value::Number),
        Yaml::Integer(i) => i.into(),
        Yaml::String(s) => s.into(),
        Yaml::Boolean(b) => b.into(),
        Yaml::Array(a) => a.into_iter().map(json).collect(),
        Yaml::Hash(h) => Value::Object(
            h.into_iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        Yaml::String(s) | Yaml::Real(s) => s,
                        Yaml::Integer(i) => i.to_string(),
                        Yaml::Boolean(b) => b.to_string(),
                        _ => return None,
                    };
                    Some((key, json(v)))
                })
                .collect(),
        ),
        _ => Value::Null,
    }
}

/// What the front matter says for `key`, written as it is or in lower
/// case.
pub fn lookup<'a>(front: &'a Value, key: &str) -> Option<&'a Value> {
    front.get(key).or_else(|| front.get(key.to_lowercase()))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// "append-head-extra" as "appendHeadExtra".
fn camelize(s: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in s.chars() {
        if c == '-' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Format `format`'s `options` with the front matter's over them, as
/// Typora loads them for an export. A key may be written "typora-key",
/// "key" or "Key"; "unset" empties a setting, as does a key left blank.
pub fn resolve(format: &str, options: &Value, front: &Value) -> Map<String, Value> {
    let mut out = options.as_object().cloned().unwrap_or_default();
    let per_file = out.get("allowPerFileSetting").and_then(Value::as_bool).unwrap_or(false);
    let keys = ALWAYS.iter().chain(if per_file { PER_FILE } else { &[] });
    for key in keys {
        // PDF and Epub take no markup from the file.
        if matches!(format, "pdf" | "epub") && key.starts_with("append-") {
            continue;
        }
        let names = [format!("typora-{key}"), key.to_string(), capitalize(key)];
        let Some(value) = names.iter().find_map(|n| lookup(front, n)) else { continue };
        let value = match value {
            Value::Null => "".into(),
            Value::String(s) if *key == "sidebar" && (s == "true" || s == "false") => (s == "true").into(),
            v => v.clone(),
        };
        out.insert(camelize(key), value);
    }
    for v in out.values_mut() {
        if v.as_str() == Some("unset") {
            *v = "".into();
        }
    }
    out
}

/// A string option, or "" when it is missing or not text.
pub fn text<'a>(options: &'a Map<String, Value>, key: &str) -> &'a str {
    options.get(key).and_then(Value::as_str).unwrap_or("")
}

/// A number option, which Preferences may have kept as text.
pub fn number(options: &Map<String, Value>, key: &str) -> Option<f64> {
    match options.get(key)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

pub fn flag(options: &Map<String, Value>, key: &str) -> bool {
    match options.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "true",
        _ => false,
    }
}

/// Today's date as `${today}` gives it: yyyy-mm-dd, in UTC.
pub fn today() -> String {
    let days = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() / 86_400) as i64;
    let (y, m, d) = civil(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The calendar date `days` after 1970-01-01 (Howard Hinnant's method).
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// How a variable's value goes into the text around it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    Text,
    /// Markup: escaped.
    Html,
    /// A shell command: a value from the document or the options can't
    /// break out of the command, so its quotes go and it is quoted
    /// unless the command already quotes it.
    Command,
}

/// What `${…}` variables read: the files, today's date, the front matter
/// and the format's options.
pub struct Vars<'a> {
    pub front: &'a Value,
    pub options: &'a Map<String, Value>,
    /// The exported file.
    pub output: &'a Path,
    /// The document's file and the folder open beside it.
    pub current: Option<&'a Path>,
    pub opened: &'a str,
    pub today: &'a str,
}

fn name(p: Option<&Path>) -> String {
    p.and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// A file's name up to its first dot.
fn stem(p: Option<&Path>) -> String {
    let n = name(p);
    n.split_once('.').map_or(n.clone(), |(s, _)| s.to_string())
}

fn path(p: Option<&Path>) -> String {
    p.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
}

/// A value as text: a list's items joined, a string's first line, and
/// "unset" nothing. None for what has no text (a mapping, null).
fn shown(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if s == "unset" => Some(String::new()),
        Value::String(s) => Some(s.lines().next().unwrap_or("").to_string()),
        Value::Array(a) => Some(a.iter().filter_map(shown).collect::<Vec<_>>().join(", ")),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

impl Vars<'_> {
    /// The built-in variable `name` (in lower case).
    fn builtin(&self, var: &str) -> Option<String> {
        let output = Some(self.output);
        Some(match var {
            "today" => self.today.to_string(),
            "outputpath" => path(output),
            "outputfolder" | "outputfolderpath" => path(self.output.parent()),
            "outputfoldername" => name_of_folder(self.output),
            "outputfilename" => stem(output),
            "outputfilefullname" => name(output),
            "currentpath" => path(self.current),
            "currentfolder" => path(self.current.and_then(Path::parent)),
            "currentfilename" => stem(self.current),
            "currentfilefullname" => name(self.current),
            "openedfolder" => self.opened.to_string(),
            _ => return None,
        })
    }

    /// A variable from `extra`, the front matter or the options, as
    /// written or in lower case; `a.b` reaches into the front matter.
    fn value(&self, var: &str, extra: &[(&str, String)]) -> Option<String> {
        let lower = var.to_lowercase();
        let from_extra = extra.iter().find(|(k, _)| *k == var || *k == lower).map(|(_, v)| v.clone());
        if from_extra.is_some() {
            return from_extra;
        }
        let found = [self.front.get(var), self.front.get(&lower), self.options.get(var), self.options.get(&lower)].into_iter().flatten().next();
        if let Some(v) = found.and_then(shown) {
            return Some(v);
        }
        if !lower.contains('.') {
            return None;
        }
        let mut at = self.front;
        for part in var.split('.') {
            at = lookup(at, part)?;
        }
        shown(at)
    }

    /// `text` with its variables filled in. One with no value stays as
    /// written, or becomes `fallback`. `extra` comes before the rest.
    pub fn expand(&self, text: &str, extra: &[(&str, String)], into: Fill, fallback: Option<&str>) -> String {
        let out = VARIABLE.replace_all(text, |c: &regex::Captures| {
            let var = &c[1];
            if let Some(v) = self.builtin(&var.to_lowercase()) {
                return if into == Fill::Html { html_escape::encode_double_quoted_attribute(&v).into_owned() } else { v };
            }
            match self.value(var, extra) {
                Some(v) => match into {
                    Fill::Text => v,
                    Fill::Html => html_escape::encode_double_quoted_attribute(&v).into_owned(),
                    Fill::Command => format!("\u{E033}{}\u{E034}", v.replace(['\'', '"', '`', '$', '\\'], "")),
                },
                None => fallback.map_or_else(|| c[0].to_string(), str::to_string),
            }
        });
        if into == Fill::Command { quote_values(&out) } else { out.into_owned() }
    }

    /// Markup appended to an HTML page, as Typora adds it: variables are
    /// filled in only in a `<meta>`'s content and the `<title>`, so a
    /// value can't add markup. Also whether it has a title of its own.
    pub fn markup(&self, html: &str) -> (String, bool) {
        let html = META.replace_all(html, |m: &regex::Captures| {
            CONTENT
                .replace_all(&m[0], |c: &regex::Captures| {
                    let quoted = &c[2];
                    let inner = &quoted[1..quoted.len() - 1];
                    format!("{}\"{}\"", &c[1], self.expand(inner, &[], Fill::Html, Some("")))
                })
                .into_owned()
        });
        let titled = TITLE.is_match(&html);
        let html = TITLE.replace_all(&html, |c: &regex::Captures| format!("{}{}{}", &c[1], self.expand(&c[2], &[], Fill::Html, None), &c[3]));
        (html.into_owned(), titled)
    }
}

/// A command with its values marked: inside quotes a value stays as it
/// is, elsewhere it is double-quoted.
fn quote_values(command: &str) -> String {
    let mut out = String::with_capacity(command.len());
    let mut open: Option<char> = None;
    for c in command.chars() {
        match (open, c) {
            (Some(_), '\u{E033}' | '\u{E034}') => {}
            (None, '\u{E033}' | '\u{E034}') => out.push('"'),
            (None, '\'' | '"') => {
                open = Some(c);
                out.push(c);
            }
            (Some(q), c) if c == q => {
                open = None;
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn name_of_folder(p: &Path) -> String {
    name(p.parent())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md;

    fn front(src: &str) -> Value {
        front_matter(&md::parse(src))
    }

    #[test]
    fn reads_the_front_matter() {
        let f = front("---\ntitle: Notes\ntags:\n  - a\n  - b\nmeta:\n  Owner: Kim\nn: 3\n---\n\n# x\n");
        assert_eq!(f["title"], "Notes");
        assert_eq!(f["tags"], serde_json::json!(["a", "b"]));
        assert_eq!(f["meta"]["Owner"], "Kim");
        assert_eq!(f["n"], 3);
        assert_eq!(front("# none\n"), serde_json::json!({}));
        assert_eq!(front("---\n- a list\n---\n"), serde_json::json!({}));
    }

    #[test]
    fn the_front_matter_overrides_what_it_may() {
        let opts = serde_json::json!({"header": "H", "footer": "F", "author": "Me", "appendHead": "<meta>"});
        let f = front("---\ntypora-header: Mine\nFooter: Theirs\nAuthor: Kim\nappend-head: <script></script>\nsidebar: 'true'\n---\n");
        // Without per-file settings, only the metadata.
        let r = resolve("html", &opts, &f);
        assert_eq!((text(&r, "header"), text(&r, "footer"), text(&r, "author")), ("H", "F", "Kim"));
        let opts = serde_json::json!({"header": "H", "footer": "F", "allowPerFileSetting": true, "appendHead": "<meta>"});
        let r = resolve("html", &opts, &f);
        assert_eq!((text(&r, "header"), text(&r, "footer"), text(&r, "appendHead")), ("Mine", "Theirs", "<script></script>"));
        assert_eq!(r["sidebar"], true);
        // A PDF takes no markup from the file; "unset" and blank empty.
        let r = resolve("pdf", &opts, &front("---\nappend-head: x\nheader: unset\nfooter:\n---\n"));
        assert_eq!((text(&r, "appendHead"), text(&r, "header"), text(&r, "footer")), ("<meta>", "", ""));
    }

    fn vars<'a>(front: &'a Value, options: &'a Map<String, Value>) -> Vars<'a> {
        Vars {
            front,
            options,
            output: Path::new("/out/dir/report.v2.pdf"),
            current: Some(Path::new("/docs/notes.md")),
            opened: "/docs",
            today: "2026-10-13",
        }
    }

    #[test]
    fn fills_in_variables() {
        let f = front("---\ntitle: T\ntags: [a, b]\nlong: |\n  one\n  two\nmeta:\n  owner: Kim\n---\n");
        let o = serde_json::json!({"author": "Me"}).as_object().cloned().unwrap();
        let v = vars(&f, &o);
        assert_eq!(
            v.expand("${title} ${Title} ${author} ${tags} ${long} ${meta.owner} ${nope} ${today}", &[], Fill::Text, None),
            "T T Me a, b one Kim ${nope} 2026-10-13"
        );
        assert_eq!(
            v.expand("${outputPath} ${outputFolder} ${outputFolderName} ${outputFileName} ${outputFileFullName}", &[], Fill::Text, None),
            "/out/dir/report.v2.pdf /out/dir dir report report.v2.pdf"
        );
        assert_eq!(
            v.expand("${currentPath} ${currentFolder} ${currentFileName} ${currentFileFullName} ${openedFolder}", &[], Fill::Text, None),
            "/docs/notes.md /docs notes notes.md /docs"
        );
        assert_eq!(v.expand("${pageNo}/${x}", &[("pageno", "3".into())], Fill::Text, Some("")), "3/");
    }

    #[test]
    fn keeps_values_out_of_commands() {
        let f = front("---\ntitle: a; rm -rf \"$HOME\"\nplain: word\n---\n");
        let o = Map::new();
        let v = vars(&f, &o);
        assert_eq!(v.expand("echo ${title}", &[], Fill::Command, None), "echo \"a; rm -rf HOME\"");
        assert_eq!(v.expand("echo \"${title}\" ${plain}", &[], Fill::Command, None), "echo \"a; rm -rf HOME\" \"word\"");
        assert_eq!(v.expand("cp \"${outputPath}\" x", &[], Fill::Command, None), "cp \"/out/dir/report.v2.pdf\" x");
    }

    #[test]
    fn fills_in_appended_markup() {
        let f = front("---\ntitle: <b>T</b>\nauthor: Kim\n---\n");
        let o = Map::new();
        let v = vars(&f, &o);
        let (html, titled) = v.markup("<meta name=\"author\" content=\"${author}\"><meta content='${x}'><p>${author}</p>");
        assert_eq!(html, "<meta name=\"author\" content=\"Kim\"><meta content=\"\"><p>${author}</p>");
        assert!(!titled);
        let (html, titled) = v.markup("<title>${title} — site</title>");
        assert_eq!(html, "<title>&lt;b&gt;T&lt;/b&gt; — site</title>");
        assert!(titled);
    }

    #[test]
    fn dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_739), (2026, 10, 13));
        assert_eq!(civil(11_016), (2000, 2, 29));
        assert_eq!(today().len(), 10);
    }
}
