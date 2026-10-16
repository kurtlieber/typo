//! Document → HTML. The output has the same shape pulldown-cmark writes, so
//! the CommonMark/GFM spec suite can check the block and inline parsers
//! against it; export and copy-as-HTML build on this.

use std::collections::{HashMap, HashSet};

use pulldown_cmark_escape::{escape_href, escape_html, escape_html_body_text};

use super::inline::{self, Kind, Node, Options, Refs};
use super::model::{Align, Block, BlockKind, Container, ContainerKind, Document, Id, ListMarker};

pub fn render(doc: &Document, opts: &Options) -> String {
    render_math(doc, opts, None)
}

/// `render`, with math drawn by `math` where it draws it (Preferences ▸
/// Markdown ▸ Math: copied as rendered SVG).
pub fn render_math(doc: &Document, opts: &Options, math: Option<&dyn Fn(&str, bool) -> Option<String>>) -> String {
    let mut r = Renderer {
        opts,
        refs: inline::refs_of(doc),
        footnotes: footnote_labels(doc),
        numbers: HashMap::new(),
        task: None,
        math,
        out: String::new(),
    };
    r.document(doc);
    r.out
}

/// One block's inline markdown as HTML, as Typora's TOC and outline show a
/// heading.
pub fn inline_html(src: &str, refs: &Refs, opts: &Options) -> String {
    let mut r = Renderer {
        opts,
        refs: refs.clone(),
        footnotes: HashSet::new(),
        numbers: HashMap::new(),
        task: None,
        math: None,
        out: String::new(),
    };
    r.inline_text(src);
    r.out
}

/// The labels of the document's footnote definitions, case-folded.
fn footnote_labels(doc: &Document) -> HashSet<String> {
    let mut labels = HashSet::new();
    for b in &doc.blocks {
        for c in &b.containers {
            if let ContainerKind::Footnote { label } = &c.kind {
                labels.insert(label.to_lowercase());
            }
        }
    }
    labels
}

struct Renderer<'a> {
    opts: &'a Options,
    refs: Refs,
    footnotes: HashSet<String>,
    /// Footnote numbers, in order of first reference or definition.
    numbers: HashMap<String, usize>,
    /// A task item's checkbox, written at the start of its first paragraph.
    task: Option<bool>,
    /// Math drawn (as SVG) instead of written as TeX, when it can be: the
    /// TeX and whether it is display math.
    math: Option<&'a dyn Fn(&str, bool) -> Option<String>>,
    out: String,
}

impl Renderer<'_> {
    fn document(&mut self, doc: &Document) {
        let mut open: Vec<&Container> = Vec::new();
        for b in &doc.blocks {
            let common = open.iter().zip(&b.containers).take_while(|(a, c)| a.id == c.id).count();
            // A list stays open when the next block starts its next item.
            let mut continuing: Option<Id> = None;
            while open.len() > common {
                let c = open.pop().unwrap();
                let next = b.containers.get(open.len()).and_then(Container::list_item);
                continuing = self.close(c, open.len() == common, next.map(|n| n.list));
            }
            for (depth, c) in b.containers.iter().enumerate().skip(common) {
                self.open(c, depth == common && continuing.is_some_and(|l| c.list_item().is_some_and(|i| i.list == l)));
                open.push(c);
            }
            self.block(b);
        }
        while let Some(c) = open.pop() {
            self.close(c, false, None);
        }
    }

    /// Close a container; returns the list id when its list carries on.
    fn close(&mut self, c: &Container, at_common: bool, next_list: Option<Id>) -> Option<Id> {
        match &c.kind {
            ContainerKind::Quote { .. } => self.out.push_str("</blockquote>\n"),
            ContainerKind::Footnote { .. } => self.out.push_str("</div>\n"),
            ContainerKind::ListItem(item) => {
                self.task = None;
                self.out.push_str("</li>\n");
                if at_common && next_list == Some(item.list) {
                    return Some(item.list);
                }
                self.out.push_str(if item.marker.is_ordered() { "</ol>\n" } else { "</ul>\n" });
            }
        }
        None
    }

    fn open(&mut self, c: &Container, list_open: bool) {
        match &c.kind {
            ContainerKind::Quote { .. } => {
                self.newline();
                self.out.push_str("<blockquote>\n");
            }
            ContainerKind::Footnote { label } => {
                self.newline();
                self.out.push_str("<div class=\"footnote-definition\" id=\"");
                self.attr(label);
                let n = self.number(label);
                self.out.push_str(&format!("\"><sup class=\"footnote-definition-label\">{n}</sup>"));
            }
            ContainerKind::ListItem(item) => {
                if !list_open {
                    self.newline();
                    match item.marker {
                        ListMarker::Bullet(_) => self.out.push_str("<ul>\n"),
                        ListMarker::Ordered { start: 1, .. } => self.out.push_str("<ol>\n"),
                        ListMarker::Ordered { start, .. } => self.out.push_str(&format!("<ol start=\"{start}\">\n")),
                    }
                }
                self.newline();
                self.out.push_str("<li>");
                self.task = item.task;
            }
        }
    }

    fn block(&mut self, b: &Block) {
        if !matches!(b.kind, BlockKind::Paragraph) {
            self.checkbox();
        }
        match &b.kind {
            BlockKind::Paragraph => self.paragraph(b),
            BlockKind::Heading { level, .. } => {
                self.newline();
                self.out.push_str(&format!("<h{level}>"));
                self.inline_text(&b.text);
                self.out.push_str(&format!("</h{level}>\n"));
            }
            BlockKind::CodeFence { info, closed, .. } => {
                self.newline();
                let info = inline::unescape(info.trim());
                let lang = info.split(' ').next().unwrap_or("");
                if lang.is_empty() {
                    self.out.push_str("<pre><code>");
                } else {
                    self.out.push_str("<pre><code class=\"language-");
                    self.attr(lang);
                    self.out.push_str("\">");
                }
                // An empty `text` is either no lines or one empty line.
                let lines = b.raw.as_ref().map_or(0, |r| r.len().saturating_sub(1 + *closed as usize));
                self.code_body(&b.text, lines);
                self.out.push_str("</code></pre>\n");
            }
            BlockKind::IndentedCode => {
                self.newline();
                self.out.push_str("<pre><code>");
                self.code_body(&b.text, 1);
                self.out.push_str("</code></pre>\n");
            }
            BlockKind::Math { .. } => {
                self.newline();
                self.out.push_str("<p><span class=\"math math-display\">");
                match self.math.and_then(|draw| draw(&b.text, true)) {
                    Some(svg) => self.out.push_str(&svg),
                    None => self.attr(&b.text),
                }
                self.out.push_str("</span></p>\n");
            }
            BlockKind::Html => {
                self.newline();
                self.out.push_str(&b.text);
                self.out.push('\n');
            }
            BlockKind::Table { align, rows } => self.table(align, rows),
            BlockKind::ThematicBreak { .. } => {
                self.newline();
                self.out.push_str("<hr />\n");
            }
            BlockKind::Toc => {
                self.newline();
                self.out.push_str("<div class=\"md-toc\"></div>\n");
            }
            BlockKind::FrontMatter => {}
            BlockKind::LinkRefDef => {
                if inline::parse_link_ref_def(&b.text).is_none() {
                    self.paragraph(b);
                }
            }
        }
    }

    fn paragraph(&mut self, b: &Block) {
        if b.text.is_empty() {
            return;
        }
        // Paragraphs directly inside a tight list item are written bare.
        let tight = b.containers.last().and_then(Container::list_item).is_some_and(|i| !i.loose);
        if tight {
            self.checkbox();
            self.inline_text(&b.text);
        } else {
            self.newline();
            self.out.push_str("<p>");
            self.checkbox();
            self.inline_text(&b.text);
            self.out.push_str("</p>\n");
        }
    }

    fn checkbox(&mut self) {
        match self.task.take() {
            Some(true) => self.out.push_str("<input disabled=\"\" type=\"checkbox\" checked=\"\"/>\n"),
            Some(false) => self.out.push_str("<input disabled=\"\" type=\"checkbox\"/>\n"),
            None => {}
        }
    }

    fn code_body(&mut self, text: &str, empty_lines: usize) {
        if text.is_empty() {
            self.out.push_str(&"\n".repeat(empty_lines.min(1)));
        } else {
            self.body(text);
            self.out.push('\n');
        }
    }

    fn table(&mut self, align: &[Align], rows: &[Vec<String>]) {
        self.newline();
        self.out.push_str("<table><thead><tr>");
        let columns = align.len();
        for (r, row) in rows.iter().enumerate() {
            let tag = if r == 0 { "th" } else { "td" };
            if r > 0 {
                self.out.push_str("<tr>");
            }
            for col in 0..columns {
                self.out.push_str(&format!("<{tag}"));
                self.out.push_str(match align[col] {
                    Align::Left => " style=\"text-align: left\">",
                    Align::Center => " style=\"text-align: center\">",
                    Align::Right => " style=\"text-align: right\">",
                    Align::None => ">",
                });
                if let Some(cell) = row.get(col) {
                    // Escaped pipes are plain pipes, even inside code spans.
                    self.inline_text(&cell.replace("\\|", "|"));
                }
                self.out.push_str(&format!("</{tag}>"));
            }
            self.out.push_str(if r == 0 { "</tr></thead><tbody>\n" } else { "</tr>\n" });
        }
        self.out.push_str("</tbody></table>\n");
    }

    // -- inline -------------------------------------------------------------

    fn inline_text(&mut self, src: &str) {
        let nodes = inline::parse(src, &self.refs, self.opts);
        self.inlines(src, &nodes);
    }

    fn inlines(&mut self, src: &str, nodes: &[Node]) {
        for n in nodes {
            let content = &src[n.content.clone()];
            match &n.kind {
                Kind::Text | Kind::Escape => self.body(content),
                Kind::Entity(s) => self.body(s),
                Kind::Code => {
                    self.out.push_str("<code>");
                    self.body(&code_text(content));
                    self.out.push_str("</code>");
                }
                Kind::Emph => self.wrap("em", src, n),
                Kind::Strong => self.wrap("strong", src, n),
                Kind::Strike => self.wrap("del", src, n),
                Kind::Highlight => self.wrap("mark", src, n),
                Kind::Sup => self.script("sup", src, n),
                Kind::Sub => self.script("sub", src, n),
                Kind::Math => {
                    self.out.push_str("<span class=\"math math-inline\">");
                    match self.math.and_then(|draw| draw(content, false)) {
                        Some(svg) => self.out.push_str(&svg),
                        None => self.attr(content),
                    }
                    self.out.push_str("</span>");
                }
                Kind::Link { dest, title } => {
                    self.out.push_str("<a href=\"");
                    self.href(dest);
                    self.title(title);
                    self.out.push_str("\">");
                    self.inlines(src, &n.children);
                    self.out.push_str("</a>");
                }
                Kind::Image { dest, title } => {
                    self.out.push_str("<img src=\"");
                    self.href(dest);
                    self.out.push_str("\" alt=\"");
                    let mut alt = String::new();
                    self.alt_text(src, &n.children, &mut alt);
                    self.attr(&alt);
                    self.title(title);
                    self.out.push_str("\" />");
                }
                Kind::Autolink { dest } => {
                    self.out.push_str("<a href=\"");
                    self.href(dest);
                    self.out.push_str("\">");
                    self.body(content);
                    self.out.push_str("</a>");
                }
                Kind::FootnoteRef { label } => {
                    if self.footnotes.contains(&label.to_lowercase()) {
                        self.out.push_str("<sup class=\"footnote-reference\"><a href=\"#");
                        self.attr(label);
                        let n = self.number(label);
                        self.out.push_str(&format!("\">{n}</a></sup>"));
                    } else {
                        self.body(&src[n.range.clone()]);
                    }
                }
                Kind::HtmlTag { .. } | Kind::Html | Kind::Comment | Kind::ImgTag(_) => self.out.push_str(&src[n.range.clone()]),
                Kind::Tag { .. } => {
                    self.out.push_str(&src[n.range.start..n.content.start]);
                    self.inlines(src, &n.children);
                    self.out.push_str(&src[n.content.end..n.range.end]);
                }
                Kind::Emoji(e) => self.out.push_str(e),
                Kind::Smart(c) => self.out.push(*c),
                Kind::SoftBreak => self.out.push('\n'),
                Kind::HardBreak => self.out.push_str("<br />\n"),
            }
        }
    }

    fn wrap(&mut self, tag: &str, src: &str, n: &Node) {
        self.out.push_str(&format!("<{tag}>"));
        self.inlines(src, &n.children);
        self.out.push_str(&format!("</{tag}>"));
    }

    /// Sub/superscript, where `\ ` is an escaped space.
    fn script(&mut self, tag: &str, src: &str, n: &Node) {
        self.out.push_str(&format!("<{tag}>"));
        for c in &n.children {
            if c.kind == Kind::Text {
                self.body(&src[c.content.clone()].replace("\\ ", " "));
            } else {
                self.inlines(src, std::slice::from_ref(c));
            }
        }
        self.out.push_str(&format!("</{tag}>"));
    }

    /// An image's alt text: its content as plain text, raw HTML included.
    fn alt_text(&mut self, src: &str, nodes: &[Node], out: &mut String) {
        for n in nodes {
            let content = &src[n.content.clone()];
            match &n.kind {
                Kind::Text | Kind::Escape | Kind::Autolink { .. } => out.push_str(content),
                Kind::Code => out.push_str(&code_text(content)),
                Kind::Entity(s) => out.push_str(s),
                Kind::Math => out.push_str(&src[n.range.clone()]),
                Kind::HtmlTag { .. } | Kind::Html | Kind::Comment => out.push_str(&src[n.range.clone()]),
                Kind::Tag { .. } => {
                    out.push_str(&src[n.range.start..n.content.start]);
                    self.alt_text(src, &n.children, out);
                    out.push_str(&src[n.content.end..n.range.end]);
                }
                Kind::FootnoteRef { label } => {
                    if self.footnotes.contains(&label.to_lowercase()) {
                        let n = self.number(label);
                        out.push_str(&format!("[{n}]"));
                    } else {
                        out.push_str(&src[n.range.clone()]);
                    }
                }
                Kind::Emoji(e) => out.push_str(e),
                Kind::Smart(c) => out.push(*c),
                Kind::SoftBreak | Kind::HardBreak => out.push(' '),
                _ => self.alt_text(src, &n.children, out),
            }
        }
    }

    fn title(&mut self, title: &str) {
        if !title.is_empty() {
            self.out.push_str("\" title=\"");
            self.attr(title);
        }
    }

    fn number(&mut self, label: &str) -> usize {
        let next = self.numbers.len() + 1;
        *self.numbers.entry(label.to_string()).or_insert(next)
    }

    fn newline(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
    }

    fn body(&mut self, s: &str) {
        let _ = escape_html_body_text(&mut self.out, s);
    }

    fn attr(&mut self, s: &str) {
        let _ = escape_html(&mut self.out, s);
    }

    fn href(&mut self, s: &str) {
        let _ = escape_href(&mut self.out, s);
    }
}

/// A code span's text: line breaks become spaces, and one space is stripped
/// from each end when both ends have one (so `` ` `` `` shows a backtick).
fn code_text(s: &str) -> String {
    let s = s.replace('\n', " ");
    if s.len() >= 2 && s.starts_with(' ') && s.ends_with(' ') && !s.trim_matches(' ').is_empty() {
        s[1..s.len() - 1].to_string()
    } else {
        s
    }
}
