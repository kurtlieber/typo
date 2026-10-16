//! File → Export → Word, OpenOffice, RTF, Epub, LaTeX, Media Wiki,
//! reStructuredText, Textile and OPML, and File → Import…: pandoc converts.
//! Typora hands pandoc its own reading of the document rather than the
//! markdown, so pandoc gets what the editor shows; typo does the same,
//! writing pandoc's JSON AST from the export tree.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{LazyLock, RwLock};

use regex::Regex;

use serde_json::{Value, json};

use super::tree::{self, Node as Part, Tree};
use super::{Equations, Look, fence_lang};
use crate::md::inline::{self, Kind, Node, Options, Refs};
use crate::md::{Align, Block, BlockKind, Document, ListMarker};
use crate::{eqnum, image};

/// `\label{…}`, which only LaTeX understands.
static LABEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\\label\s*\{[^{}]*\}").unwrap());
/// A metadata string that is TeX: it starts with a command.
static TEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\\[a-zA-Z]").unwrap());

/// Preferences ▸ Export ▸ General ▸ Pandoc Path: the pandoc to run, or
/// "" for the one on PATH.
static PROGRAM: RwLock<String> = RwLock::new(String::new());

pub fn set_program(path: &str) {
    *PROGRAM.write().unwrap_or_else(|e| e.into_inner()) = path.trim().to_string();
}

fn pandoc() -> Command {
    let program = PROGRAM.read().unwrap_or_else(|e| e.into_inner()).clone();
    Command::new(if program.is_empty() { "pandoc".into() } else { program })
}

/// `args` split as a shell would split them: at spaces outside quotes,
/// with quotes and backslashes taken out.
pub fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word: Option<String> = None;
    let mut quote: Option<char> = None;
    let mut chars = args.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => word.get_or_insert_default().extend(chars.next()),
            (Some(_), c) => word.get_or_insert_default().push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                word.get_or_insert_default();
            }
            (None, '\\') => word.get_or_insert_default().extend(chars.next()),
            (None, c) if c.is_whitespace() => out.extend(word.take()),
            (None, c) => word.get_or_insert_default().push(c),
        }
    }
    out.extend(word);
    out
}

/// pandoc's name for an export format, as the Export menu names it.
pub fn target(format: &str) -> &str {
    match format {
        "mediawiki" | "wiki" => "mediawiki",
        f => f,
    }
}

/// Typora asks for pandoc 2.0 or later.
pub fn available() -> bool {
    version().is_some_and(|v| v.first().is_some_and(|&major| major >= 2))
}

fn version() -> Option<Vec<u64>> {
    let out = pandoc().arg("--version").stderr(Stdio::null()).output().ok()?;
    let first = String::from_utf8_lossy(&out.stdout).lines().next()?.to_string();
    let v = first.split_whitespace().nth(1)?;
    Some(v.split('.').filter_map(|n| n.parse().ok()).collect())
}

/// The AST version this pandoc reads, from the JSON it writes.
fn api_version() -> Result<Value, String> {
    let out = run(&["-f", "markdown", "-t", "json"], Some(b""), None)?;
    let v: Value = serde_json::from_slice(&out).map_err(|e| e.to_string())?;
    Ok(v["pandoc-api-version"].clone())
}

/// Run pandoc with `args`, feeding it `input`, from folder `cwd`: what it
/// wrote, or what it said went wrong.
fn run(args: &[&str], input: Option<&[u8]>, cwd: Option<&Path>) -> Result<Vec<u8>, String> {
    let mut cmd = pandoc();
    cmd.args(args).stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = cwd.filter(|d| d.is_dir()) {
        cmd.current_dir(dir);
    }
    let mut child = cmd.spawn().map_err(|e| format!("Could not run pandoc: {e}"))?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        let input = input.to_vec();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() { format!("pandoc failed ({})", out.status) } else { err });
    }
    Ok(out.stdout)
}

/// One conversion: where the document is from and goes, what pandoc is
/// told besides, and the front matter, which becomes its metadata.
pub struct Run<'a> {
    pub format: &'a str,
    pub file: Option<&'a Path>,
    pub image_root: Option<&'a Path>,
    pub out: &'a Path,
    /// After Typora's own arguments: the format's options and its Extra
    /// Arguments.
    pub args: &'a [String],
    pub meta: &'a Value,
}

/// Convert the document as `run` says, as Typora runs pandoc: `-f native
/// -s -o out -t format` and the rest, from the document's folder.
pub fn export(doc: &Document, refs: &Refs, opts: &Options, look: &Look, run: &Run) -> Result<(), String> {
    let target = target(run.format);
    let mut meta = run.meta.clone();
    // An Epub needs a title: the file's name, as words.
    if run.format == "epub" && meta.get("title").is_none() {
        let name = run.file.and_then(Path::file_stem).map(|s| s.to_string_lossy().replace('-', " ")).unwrap_or_default();
        meta["title"] = capitalize_words(&name).into();
    }
    let mut ast = ast(doc, refs, opts, look, target, run.file, run.image_root, &meta);
    ast["pandoc-api-version"] = api_version()?;
    let out = run.out.to_string_lossy();
    let mut args = vec!["-f", "json", "-s", "-o", &out, "-t", target];
    args.extend(run.args.iter().map(String::as_str));
    self::run(&args, Some(ast.to_string().as_bytes()), run.file.and_then(Path::parent)).map(|_| ())
}

fn capitalize_words(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// File → Import…: `path` as markdown, as Typora asks pandoc for it.
pub fn import(path: &Path) -> Result<String, String> {
    let file = path.to_string_lossy();
    let args = [&*file, "--quiet", "--wrap=none", "-t", "gfm-smart+tex_math_dollars-fenced_divs-bracketed_spans-raw_html"];
    let out = run(&args, None, path.parent())?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// The document as pandoc's AST, without its version, with `meta` (the
/// front matter, a mapping) as its metadata.
#[allow(clippy::too_many_arguments)]
pub fn ast(doc: &Document, refs: &Refs, opts: &Options, look: &Look, target: &str, file: Option<&Path>, image_root: Option<&Path>, meta: &Value) -> Value {
    let tree = tree::build(doc, refs, opts);
    let equations = look.equations(doc);
    let w = Writer { doc, refs, opts, look, target, file, image_root, tree: &tree, equations: &equations };
    let meta: serde_json::Map<String, Value> =
        meta.as_object().into_iter().flatten().filter_map(|(k, v)| Some((k.clone(), w.meta(k, v)?))).collect();
    json!({"meta": meta, "blocks": w.nodes(&tree.body, false)})
}

struct Writer<'a> {
    doc: &'a Document,
    refs: &'a Refs,
    opts: &'a Options,
    look: &'a Look,
    target: &'a str,
    file: Option<&'a Path>,
    image_root: Option<&'a Path>,
    tree: &'a Tree<'a>,
    equations: &'a Equations,
}

fn el(t: &str, c: Value) -> Value {
    json!({"t": t, "c": c})
}

fn tag(t: &str) -> Value {
    json!({"t": t})
}

fn attr(id: &str, classes: &[&str]) -> Value {
    json!([id, classes, []])
}

/// Text as pandoc words: `Str`s with a `Space` between.
fn words(s: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for (i, w) in s.split(' ').enumerate() {
        if i > 0 && !matches!(out.last(), Some(v) if *v == tag("Space")) {
            out.push(tag("Space"));
        }
        if !w.is_empty() {
            out.push(el("Str", json!(w)));
        }
    }
    out
}

impl Writer<'_> {
    fn nodes(&self, nodes: &[Part], tight: bool) -> Vec<Value> {
        let mut out = Vec::new();
        for n in nodes {
            match n {
                Part::Block { index, block, skip } => out.extend(self.block(*index, block, *skip, tight)),
                Part::Quote { alert: None, children } => out.push(el("BlockQuote", json!(self.nodes(children, false)))),
                // pandoc reads GitHub's alerts as a div of its kind, with a
                // title div first.
                Part::Quote { alert: Some(kind), children } => {
                    let mut title = kind.to_string();
                    title[..1].make_ascii_uppercase();
                    let mut blocks = vec![el("Div", json!([attr("", &["title"]), [el("Para", json!(words(&title)))]]))];
                    blocks.extend(self.nodes(children, false));
                    out.push(el("Div", json!([attr("", &[kind]), blocks])));
                }
                Part::List(list) => out.push(self.list(list)),
            }
        }
        out
    }

    fn list(&self, list: &tree::List) -> Value {
        let items: Vec<Value> = list
            .items
            .iter()
            .map(|item| {
                let mut blocks = self.nodes(&item.children, !list.loose);
                // A task's box leads its first paragraph, as pandoc writes
                // task lists.
                if let Some(done) = item.task {
                    let mark = [el("Str", json!(if done { "☒" } else { "☐" })), tag("Space")];
                    let text = blocks.first_mut().filter(|b| b["t"] == "Plain" || b["t"] == "Para");
                    match text.and_then(|b| b["c"].as_array_mut()) {
                        Some(inlines) => {
                            inlines.splice(0..0, mark);
                        }
                        None => blocks.insert(0, el("Plain", json!(mark[..1]))),
                    }
                }
                json!(blocks)
            })
            .collect();
        match list.marker {
            ListMarker::Bullet(_) => el("BulletList", json!(items)),
            ListMarker::Ordered { start, delim, .. } => {
                let delim = if delim == ')' { "OneParen" } else { "Period" };
                el("OrderedList", json!([[start, tag("Decimal"), tag(delim)], items]))
            }
        }
    }

    fn block(&self, index: usize, b: &Block, skip: usize, tight: bool) -> Vec<Value> {
        let para = if tight { "Plain" } else { "Para" };
        match &b.kind {
            BlockKind::Paragraph => {
                let text = &b.text[skip..];
                if text.trim().is_empty() {
                    return Vec::new();
                }
                vec![el(para, json!(self.inline_text(text)))]
            }
            BlockKind::LinkRefDef => match inline::parse_link_ref_def(&b.text) {
                Some(_) => Vec::new(),
                None => vec![el(para, json!(self.inline_text(&b.text)))],
            },
            BlockKind::Heading { level, .. } => {
                let id = self.tree.anchor(index).unwrap_or("");
                vec![el("Header", json!([level, attr(id, &[]), self.inline_text(&b.text)]))]
            }
            BlockKind::CodeFence { info, .. } if self.look.is_math(&b.kind) => {
                let _ = info;
                vec![self.display_math(&b.text)]
            }
            BlockKind::CodeFence { info, .. } => {
                let lang = fence_lang(info);
                let classes: Vec<&str> = if lang.is_empty() { vec![] } else { vec![&lang] };
                vec![el("CodeBlock", json!([attr("", &classes), b.text.trim_end_matches('\n')]))]
            }
            BlockKind::IndentedCode => vec![el("CodeBlock", json!([attr("", &[]), b.text.trim_end_matches('\n')]))],
            BlockKind::Math { .. } => vec![self.display_math(&b.text)],
            BlockKind::Html => vec![el("RawBlock", json!(["html", b.text]))],
            BlockKind::Table { align, rows } => vec![self.table(align, rows)],
            BlockKind::ThematicBreak { .. } => vec![tag("HorizontalRule")],
            BlockKind::Toc => self.toc(),
            BlockKind::FrontMatter => Vec::new(),
        }
    }

    /// Display math in a paragraph of its own. LaTeX keeps `\label` and
    /// `\ref`; elsewhere references read as the numbers the editor shows.
    fn display_math(&self, tex: &str) -> Value {
        let tex = if self.target == "latex" {
            tex.trim().to_string()
        } else {
            LABEL.replace_all(&eqnum::with_refs(tex.trim(), &self.equations.labels), "").into_owned()
        };
        el("Para", json!([el("Math", json!([tag("DisplayMath"), tex]))]))
    }

    fn table(&self, align: &[Align], rows: &[Vec<String>]) -> Value {
        let aligns: Vec<Value> = align
            .iter()
            .map(|a| {
                tag(match a {
                    Align::Left => "AlignLeft",
                    Align::Center => "AlignCenter",
                    Align::Right => "AlignRight",
                    Align::None => "AlignDefault",
                })
            })
            .collect();
        let row = |cells: &Vec<String>| -> Value {
            let cells: Vec<Value> = (0..align.len())
                .map(|c| {
                    let text = cells.get(c).map(|t| t.replace("\\|", "|")).unwrap_or_default();
                    let blocks = if text.trim().is_empty() { json!([]) } else { json!([el("Plain", json!(self.inline_text(&text)))]) };
                    json!([attr("", &[]), tag("AlignDefault"), 1, 1, blocks])
                })
                .collect();
            json!([attr("", &[]), cells])
        };
        let specs: Vec<Value> = aligns.iter().map(|a| json!([a, tag("ColWidthDefault")])).collect();
        let head: Vec<Value> = rows.first().map(row).into_iter().collect();
        let body: Vec<Value> = rows.iter().skip(1).map(row).collect();
        el(
            "Table",
            json!([
                attr("", &[]),
                [null, []],
                specs,
                [attr("", &[]), head],
                [[attr("", &[]), 0, [], body]],
                [attr("", &[]), []]
            ]),
        )
    }

    /// `[TOC]` as each format writes a table of contents, as Typora does.
    fn toc(&self) -> Vec<Value> {
        let raw = |f: &str, s: &str| el("RawBlock", json!([f, s]));
        match self.target {
            "latex" => vec![raw("latex", "\\tableofcontents")],
            "mediawiki" => vec![raw("mediawiki", "__TOC__")],
            "rst" => vec![raw("rst", ".. contents::")],
            "textile" => vec![raw("textile", "{toc}")],
            _ => Vec::new(),
        }
    }

    /// A front matter value as metadata, as Typora hands it over: a
    /// string is markdown, or TeX when it starts with a command; a
    /// relative `cover-image` is found from the document's folder.
    fn meta(&self, key: &str, v: &Value) -> Option<Value> {
        Some(match v {
            Value::String(s) if key == "cover-image" && (s.starts_with("./") || s.starts_with("../")) => {
                let dir = self.file.and_then(Path::parent).unwrap_or(Path::new(""));
                el("MetaInlines", json!([el("Str", dir.join(s.strip_prefix("./").unwrap_or(s)).to_string_lossy().into())]))
            }
            Value::String(s) if TEX.is_match(s) && s.contains('\n') => el("MetaBlocks", json!([el("RawBlock", json!(["tex", s]))])),
            Value::String(s) if TEX.is_match(s) => el("MetaInlines", json!([el("RawInline", json!(["tex", s]))])),
            Value::String(s) => el("MetaInlines", self.inline_text(s).into()),
            Value::Bool(b) => el("MetaBool", (*b).into()),
            Value::Number(n) => el("MetaString", n.to_string().into()),
            Value::Array(a) => el("MetaList", a.iter().filter_map(|v| self.meta("", v)).collect()),
            Value::Object(m) => el("MetaMap", m.iter().filter_map(|(k, v)| Some((k.clone(), self.meta(k, v)?))).collect()),
            Value::Null => return None,
        })
    }

    // -- inline -------------------------------------------------------------

    fn inline_text(&self, src: &str) -> Vec<Value> {
        let nodes = inline::parse(src, self.refs, self.opts);
        self.inlines(src, &nodes)
    }

    fn inlines(&self, src: &str, nodes: &[Node]) -> Vec<Value> {
        let mut out = Vec::new();
        for n in nodes {
            let content = &src[n.content.clone()];
            match &n.kind {
                Kind::Text | Kind::Escape => out.extend(words(content)),
                Kind::Entity(s) => out.extend(words(s)),
                Kind::Code => out.push(el("Code", json!([attr("", &[]), content]))),
                Kind::Emph => out.push(el("Emph", json!(self.inlines(src, &n.children)))),
                Kind::Strong => out.push(el("Strong", json!(self.inlines(src, &n.children)))),
                Kind::Strike => out.push(el("Strikeout", json!(self.inlines(src, &n.children)))),
                Kind::Highlight => out.push(el("Span", json!([attr("", &["mark"]), self.inlines(src, &n.children)]))),
                Kind::Sup => out.push(el("Superscript", json!(self.script(src, n)))),
                Kind::Sub => out.push(el("Subscript", json!(self.script(src, n)))),
                Kind::Math => {
                    let tex = if self.target == "latex" { content.to_string() } else { eqnum::with_refs(content, &self.equations.labels).into_owned() };
                    out.push(el("Math", json!([tag("InlineMath"), tex])));
                }
                Kind::Link { dest, title } => out.push(el("Link", json!([attr("", &[]), self.inlines(src, &n.children), [dest, title]]))),
                Kind::Image { dest, title } => {
                    let alt = self.inlines(src, &n.children);
                    out.push(el("Image", json!([attr("", &[]), alt, [self.image_src(dest), title]])));
                }
                Kind::ImgTag(t) => {
                    out.push(el("Image", json!([attr("", &[]), words(&t.alt), [self.image_src(&t.src), t.title]])));
                }
                Kind::Autolink { dest } => out.push(el("Link", json!([attr("", &[]), words(content), [dest, ""]]))),
                Kind::FootnoteRef { label } => match self.tree.footnotes.iter().find(|f| self.tree.footnote_number(label) == Some(f.number)) {
                    Some(f) => out.push(el("Note", json!(self.nodes(&f.children, false)))),
                    None => out.extend(words(&src[n.range.clone()])),
                },
                Kind::HtmlTag { .. } | Kind::Html => out.push(el("RawInline", json!(["html", &src[n.range.clone()]]))),
                Kind::Comment => {}
                Kind::Tag { name, .. } => {
                    let inner = self.inlines(src, &n.children);
                    out.push(match name.as_str() {
                        "u" | "ins" => el("Underline", json!(inner)),
                        "s" | "del" | "strike" => el("Strikeout", json!(inner)),
                        "b" | "strong" => el("Strong", json!(inner)),
                        "i" | "em" => el("Emph", json!(inner)),
                        "sup" => el("Superscript", json!(inner)),
                        "sub" => el("Subscript", json!(inner)),
                        "small" => el("SmallCaps", json!(inner)),
                        "mark" | "kbd" => el("Span", json!([attr("", &[name]), inner])),
                        _ => el("Span", json!([attr("", &[]), inner])),
                    });
                }
                Kind::Emoji(e) => out.push(el("Str", json!(e))),
                Kind::Smart(c) => out.push(el("Str", json!(c.to_string()))),
                Kind::SoftBreak => out.push(tag(if self.look.line_breaks { "LineBreak" } else { "SoftBreak" })),
                Kind::HardBreak => out.push(tag("LineBreak")),
            }
        }
        out
    }

    /// Sub/superscript, where `\ ` is an escaped space.
    fn script(&self, src: &str, n: &Node) -> Vec<Value> {
        let mut out = Vec::new();
        for c in &n.children {
            if c.kind == Kind::Text {
                out.extend(words(&src[c.content.clone()].replace("\\ ", " ")));
            } else {
                out.extend(self.inlines(src, std::slice::from_ref(c)));
            }
        }
        out
    }

    /// A local image by its full path, so pandoc finds it from anywhere.
    fn image_src(&self, src: &str) -> String {
        let written = src.trim();
        if written.contains("://") || written.starts_with("data:") {
            return src.to_string();
        }
        image::local_path(written, self.file, self.image_root).map_or_else(|| src.to_string(), |p| p.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eqnum::Numbering;
    use crate::md::parse;
    use crate::palette::Scheme;

    fn blocks(src: &str, target: &str) -> String {
        let doc = parse(src);
        let refs = inline::refs_of(&doc);
        let look = Look { scheme: Scheme::paper(), numbering: Numbering::Ams, math_fences: true, diagrams: true, ..Look::test() };
        ast(&doc, &refs, &Options::all(), &look, target, None, None, &Value::Null)["blocks"].to_string()
    }

    #[test]
    fn passes_the_front_matter_as_metadata() {
        let doc = parse("x\n");
        let refs = inline::refs_of(&doc);
        let meta = json!({"title": "A *b*", "geometry": "\\usepackage{x}", "toc": true, "tags": ["t"], "cover-image": "./c.png", "n": null});
        let look = Look::test();
        let file = Path::new("/d/f.md");
        let ast = ast(&doc, &refs, &Options::all(), &look, "epub", Some(file), None, &meta);
        let m = &ast["meta"];
        assert_eq!(m["title"].to_string(), r#"{"t":"MetaInlines","c":[{"t":"Str","c":"A"},{"t":"Space"},{"t":"Emph","c":[{"t":"Str","c":"b"}]}]}"#);
        assert_eq!(m["geometry"]["c"][0]["c"], json!(["tex", "\\usepackage{x}"]));
        assert_eq!(m["toc"], json!({"t": "MetaBool", "c": true}));
        assert_eq!(m["tags"]["t"], "MetaList");
        assert_eq!(m["cover-image"]["c"][0]["c"], "/d/c.png");
        assert!(m.get("n").is_none());
    }

    #[test]
    fn splits_arguments() {
        assert_eq!(split_args(r#" --toc  -V "title=a b" --x='c d'  e\ f "g\"h" "#), ["--toc", "-V", "title=a b", "--x=c d", "e f", "g\"h"]);
        assert_eq!(split_args(r#"''"#), [""]);
        assert!(split_args("  ").is_empty());
    }

    #[test]
    fn writes_blocks() {
        assert_eq!(
            blocks("# Hi there\n\nSome *em* `c`.\n", "docx"),
            r#"[{"t":"Header","c":[1,["hi-there",[],[]],[{"t":"Str","c":"Hi"},{"t":"Space"},{"t":"Str","c":"there"}]]},{"t":"Para","c":[{"t":"Str","c":"Some"},{"t":"Space"},{"t":"Emph","c":[{"t":"Str","c":"em"}]},{"t":"Space"},{"t":"Code","c":[["",[],[]],"c"]},{"t":"Str","c":"."}]}]"#
        );
        assert_eq!(blocks("[TOC]\n", "latex"), r#"[{"t":"RawBlock","c":["latex","\\tableofcontents"]}]"#);
        assert_eq!(blocks("[TOC]\n", "docx"), "[]");
    }

    #[test]
    fn writes_tasks_alerts_and_notes() {
        let b = blocks("- [x] done\n", "docx");
        assert!(b.contains(r#"{"t":"Plain","c":[{"t":"Str","c":"☒"},{"t":"Space"},{"t":"Str","c":"done"}]}"#), "{b}");
        let b = blocks("> [!TIP]\n> Use it.\n", "docx");
        assert!(b.starts_with(r#"[{"t":"Div","c":[["",["tip"],[]],[{"t":"Div","c":[["",["title"],[]],[{"t":"Para","c":[{"t":"Str","c":"Tip"}]}]]}"#), "{b}");
        let b = blocks("a[^n]\n\n[^n]: The note.\n", "docx");
        assert!(b.contains(r#""t":"Note""#) && !b.contains("[^n]"), "{b}");
    }

    #[test]
    fn writes_math_and_tables() {
        let b = blocks("$$\n\\begin{equation} x \\label{a} \\end{equation}\n$$\n\nSee $\\eqref{a}$.\n", "docx");
        assert!(b.contains("DisplayMath") && b.contains(r#"\\text{(1)}"#) && !b.contains("label"), "{b}");
        let b = blocks("| a | b |\n|:-|-:|\n| 1 | |\n", "docx");
        assert!(b.contains(r#"[[{"t":"AlignLeft"},{"t":"ColWidthDefault"}],[{"t":"AlignRight"},{"t":"ColWidthDefault"}]]"#), "{b}");
    }

    /// Converts through the installed pandoc, when there is one.
    #[test]
    fn converts_with_pandoc() {
        if !available() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("typo-pandoc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = "# T\n\n[TOC]\n\n- [ ] a\n\n$$\nx^2\n$$\n\n| a |\n|-|\n| 1 |\n\nNote[^n].\n\n[^n]: N.\n";
        let doc = parse(src);
        let refs = inline::refs_of(&doc);
        let look = Look { scheme: Scheme::paper(), numbering: Numbering::None, math_fences: true, diagrams: true, ..Look::test() };
        for format in ["docx", "odt", "rtf", "epub", "latex", "mediawiki", "rst", "textile", "opml"] {
            let out = dir.join(format!("t.{format}"));
            let run = Run { format, file: None, image_root: None, out: &out, args: &[], meta: &json!({"title": "T"}) };
            export(&doc, &refs, &Options::all(), &look, &run).unwrap_or_else(|e| panic!("{format}: {e}"));
            assert!(out.metadata().unwrap().len() > 0, "{format}");
        }
        let tex = std::fs::read_to_string(dir.join("t.latex")).unwrap();
        assert!(tex.contains("\\tableofcontents") && tex.contains("x^2"));
        std::fs::write(dir.join("back.html"), "<h1>Back</h1><p>A <b>b</b> and <em>e</em>.</p>").unwrap();
        assert_eq!(import(&dir.join("back.html")).unwrap(), "# Back\n\nA **b** and *e*.\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
#[ignore]
fn sample_pandoc() {
    use crate::palette::Scheme;
    let path = std::path::PathBuf::from(std::env::var("TYPO_SAMPLE").unwrap());
    let doc = crate::md::parse(&std::fs::read_to_string(&path).unwrap());
    let refs = inline::refs_of(&doc);
    let look = Look { scheme: Scheme::paper(), numbering: eqnum::Numbering::Ams, math_fences: true, diagrams: true, ..Look::test() };
    for format in ["mediawiki", "docx", "latex"] {
        let out = path.with_extension(format);
        let run = Run { format, file: Some(&path), image_root: None, out: &out, args: &[], meta: &json!({}) };
        export(&doc, &refs, &Options::all(), &look, &run).unwrap();
    }
}
