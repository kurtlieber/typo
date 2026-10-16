//! The document as an HTML page, as Typora's File → Export → HTML writes
//! it: the content in `<div id='write'>` under a stylesheet in the theme's
//! colours (css.rs), or, for "HTML (without Styles)", the bare markup. Math
//! and diagrams are inline SVG and code is highlighted in the markup, so
//! the page shows the same with no script and nothing beside it.

use std::path::Path;

use pulldown_cmark_escape::{escape_href, escape_html, escape_html_body_text};

use super::tree::{self, Node as Part, Tree};
use super::{Equations, Look, css, fence_lang};
use crate::code::{Highlight, Token};
use crate::md::inline::{self, Kind, Node, Options, Refs};
use crate::md::query::{self, ALERTS};
use crate::md::{Align, Block, BlockKind, Document, imgtag};
use crate::{diagram, eqnum, image, math};

/// What to write besides the document.
pub struct Page<'a> {
    /// The page's `<title>`: the front matter's, or the file's name. None
    /// when the markup added to `<head>` has its own.
    pub title: Option<&'a str>,
    /// Typora's "HTML", rather than "HTML (without Styles)".
    pub styled: bool,
    pub look: &'a Look,
    /// Markup the export settings add at the end of `<head>` and `<body>`.
    pub head: &'a str,
    pub body: &'a str,
    /// The document's file and image root, to find its images.
    pub file: Option<&'a Path>,
    pub image_root: Option<&'a Path>,
    /// The folder the page goes in: relative image paths are rewritten to
    /// reach the same files from there.
    pub dest: Option<&'a Path>,
    pub sidebar: Sidebar,
}

/// Preferences ▸ Export ▸ HTML ▸ Include Outline Sidebar: none, or the
/// outline as the side panel shows it, flat or collapsible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sidebar {
    None,
    Flat,
    Collapsible,
}

/// What opens and closes the sidebar's sections and marks the one being
/// read.
const SIDEBAR_SCRIPT: &str = r#"<script>(function () {
  var side = document.querySelector('.typora-export-sidebar');
  if (!side) return;
  var collapse = document.body.classList.contains('typora-export-collapse-outline');
  var links = [].slice.call(side.querySelectorAll('.outline-label'));
  side.addEventListener('click', function (e) {
    var x = e.target.closest('.outline-expander');
    if (!x) return;
    x.closest('.outline-item-wrapper').classList.toggle('outline-item-open');
    e.preventDefault();
  });
  function mark() {
    var current = null;
    links.forEach(function (a) {
      var h = document.getElementById(decodeURIComponent(a.getAttribute('href').slice(1)));
      if (h && h.getBoundingClientRect().top < 80) current = a;
    });
    side.querySelectorAll('.outline-item-active').forEach(function (w) { w.classList.remove('outline-item-active'); });
    if (!current) return;
    var w = current.closest('.outline-item-wrapper');
    w.classList.add('outline-item-active');
    for (var up = w.parentElement.closest('.outline-item-wrapper'); collapse && up; up = up.parentElement.closest('.outline-item-wrapper'))
      up.classList.add('outline-item-open');
  }
  window.addEventListener('scroll', mark, { passive: true });
  mark();
})();</script>
"#;

/// The colour math is typeset in, replaced with the text's own.
const INK: &str = "010203";

/// The outline of each alert's icon, as the editor draws it (Glyph.qml).
pub const ALERT_ICONS: [&str; 5] = [
    "M14 8 A6 6 0 1 1 2 8 A6 6 0 1 1 14 8 Z M8 7.3 V11.3 M8 4.6 V5.2",
    "M6 13.5 H10 M6.5 11 C6.5 9.5 4 8.5 4 6 A4 4 0 1 1 12 6 C12 8.5 9.5 9.5 9.5 11 Z",
    "M2.5 2.5 H13.5 V10.5 H7.5 L4.5 13.5 V10.5 H2.5 Z M8 4.8 V7.6 M8 9 V9.4",
    "M8 2 L14.5 13.5 H1.5 Z M8 6.2 V9.6 M8 11.2 V11.6",
    "M5.5 1.8 H10.5 L14.2 5.5 V10.5 L10.5 14.2 H5.5 L1.8 10.5 V5.5 Z M8 4.8 V8.8 M8 10.6 V11",
];

/// The whole page.
pub fn page(doc: &Document, refs: &Refs, opts: &Options, p: &Page) -> String {
    let (content, outline) = write(doc, refs, opts, p, p.styled && p.sidebar != Sidebar::None);
    let mut out = String::from("<!doctype html>\n<html>\n<head>\n<meta charset='UTF-8'><meta name='viewport' content='width=device-width initial-scale=1'>\n");
    if let Some(title) = p.title {
        out.push_str("<title>");
        let _ = escape_html_body_text(&mut out, title);
        out.push_str("</title>\n");
    }
    if p.styled {
        out.push_str("<style type='text/css'>\n");
        out.push_str(&css::stylesheet(&p.look.scheme));
        if !outline.is_empty() {
            out.push_str(&css::sidebar(&p.look.scheme));
        }
        out.push_str("</style>\n");
    }
    push_extra(&mut out, p.head);
    out.push_str("</head>\n");
    if p.styled {
        let mut classes = String::from("typora-export");
        if !outline.is_empty() {
            classes.push_str(" typora-export-show-outline");
            classes.push_str(if p.sidebar == Sidebar::Collapsible { " typora-export-collapse-outline" } else { " typora-export-no-collapse-outline" });
        }
        out.push_str(&format!("<body class='{classes}'>\n<div class='typora-export-content'>\n"));
        if !outline.is_empty() {
            out.push_str("<div class='typora-export-sidebar'><div class='outline-content'>");
            out.push_str(&outline);
            out.push_str("</div></div>\n");
        }
        out.push_str(if p.look.indent_first_line { "<div id='write' class='first-line-indent'>\n" } else { "<div id='write'>\n" });
        out.push_str(&content);
        out.push_str("</div>\n</div>\n");
        if !outline.is_empty() {
            out.push_str(SIDEBAR_SCRIPT);
        }
    } else {
        out.push_str("<body>\n");
        out.push_str(&content);
    }
    push_extra(&mut out, p.body);
    out.push_str("</body>\n</html>\n");
    out
}

fn push_extra(out: &mut String, extra: &str) {
    if !extra.trim().is_empty() {
        out.push_str(extra.trim_end());
        out.push('\n');
    }
}

/// The document's markup: what goes in `#write`.
#[cfg(test)]
pub fn body(doc: &Document, refs: &Refs, opts: &Options, p: &Page) -> String {
    write(doc, refs, opts, p, false).0
}

/// The document's markup, and with `outline` the sidebar's list of its
/// headings.
fn write(doc: &Document, refs: &Refs, opts: &Options, p: &Page, outline: bool) -> (String, String) {
    let tree = tree::build(doc, refs, opts);
    let equations = p.look.equations(doc);
    let mut w = Writer { doc, refs, opts, page: p, tree: &tree, equations: &equations, seen: Vec::new(), toc: false, out: String::new() };
    let list = if outline { w.outline() } else { String::new() };
    w.nodes(&tree.body, false);
    w.footnotes();
    (w.out, list)
}

struct Writer<'a> {
    doc: &'a Document,
    refs: &'a Refs,
    opts: &'a Options,
    page: &'a Page<'a>,
    tree: &'a Tree<'a>,
    equations: &'a Equations,
    /// How often each footnote has been referred to so far, by number, so
    /// each reference has an id of its own and the note links to the first.
    seen: Vec<usize>,
    /// Writing a TOC entry: links and pictures in a heading are its text.
    toc: bool,
    out: String,
}

impl Writer<'_> {
    fn nodes(&mut self, nodes: &[Part], tight: bool) {
        for n in nodes {
            match n {
                Part::Block { index, block, skip } => self.block(*index, block, *skip, tight),
                Part::Quote { alert: None, children } => {
                    self.line("<blockquote>");
                    self.nodes(children, false);
                    self.line("</blockquote>");
                }
                Part::Quote { alert: Some(kind), children } => self.alert(kind, children),
                Part::List(list) => self.list(list),
            }
        }
    }

    fn alert(&mut self, kind: &str, children: &[Part]) {
        self.line(&format!("<div class='md-alert md-alert-{kind}'>"));
        self.out.push_str(&format!("<p class='md-alert-text-container'><span class='md-alert-text md-alert-text-{kind}'>"));
        if self.page.styled
            && let Some(i) = ALERTS.iter().position(|a| *a == kind)
        {
            self.out.push_str(&format!("<svg viewBox='0 0 16 16' aria-hidden='true'><path d='{}'/></svg>", ALERT_ICONS[i]));
        }
        let mut title = kind.to_string();
        title[..1].make_ascii_uppercase();
        self.out.push_str(&format!("<span>{title}</span></span></p>\n"));
        self.nodes(children, false);
        self.line("</div>");
    }

    fn list(&mut self, list: &tree::List) {
        let tasks = list.items.iter().any(|i| i.task.is_some());
        let class = if tasks { " class='contains-task-list'" } else { "" };
        let close = match list.start {
            None => {
                self.line(&format!("<ul{class}>"));
                "</ul>"
            }
            Some(1) => {
                self.line(&format!("<ol{class}>"));
                "</ol>"
            }
            Some(n) => {
                self.line(&format!("<ol start='{n}'{class}>"));
                "</ol>"
            }
        };
        for item in &list.items {
            self.newline();
            self.out.push_str(match item.task {
                Some(true) => "<li class='task-list-item'><input type='checkbox' disabled checked />",
                Some(false) => "<li class='task-list-item'><input type='checkbox' disabled />",
                None => "<li>",
            });
            self.nodes(&item.children, !list.loose);
            self.out.push_str("</li>\n");
        }
        self.line(close);
    }

    fn block(&mut self, index: usize, b: &Block, skip: usize, tight: bool) {
        match &b.kind {
            BlockKind::Paragraph => self.paragraph(&b.text[skip..], tight),
            BlockKind::LinkRefDef => {
                if inline::parse_link_ref_def(&b.text).is_none() {
                    self.paragraph(&b.text, tight);
                }
            }
            BlockKind::Heading { level, .. } => {
                self.newline();
                self.out.push_str(&format!("<h{level}"));
                if let Some(anchor) = self.tree.anchor(index) {
                    self.out.push_str(" id='");
                    self.attr(anchor);
                    self.out.push('\'');
                }
                self.out.push('>');
                self.inline_text(&b.text);
                self.out.push_str(&format!("</h{level}>\n"));
            }
            BlockKind::CodeFence { info, .. } => {
                let lang = fence_lang(info);
                let lower = lang.to_lowercase();
                if self.page.look.is_math(&b.kind) {
                    self.display_math(index, &b.text);
                } else if self.page.look.diagrams && diagram::LANGUAGES.contains(&lower.as_str()) {
                    self.diagram(&lower, &b.text);
                } else {
                    self.code(&lang, &b.text);
                }
            }
            BlockKind::IndentedCode => self.code("", &b.text),
            BlockKind::Math { .. } => self.display_math(index, &b.text),
            BlockKind::Html => {
                self.newline();
                self.out.push_str(&b.text);
                self.out.push('\n');
            }
            BlockKind::Table { align, rows } => self.table(align, rows),
            BlockKind::ThematicBreak { .. } => self.line("<hr />"),
            BlockKind::Toc => self.toc(),
            BlockKind::FrontMatter => {}
        }
    }

    fn paragraph(&mut self, text: &str, tight: bool) {
        if text.trim().is_empty() {
            return;
        }
        let nodes = inline::parse(text, self.refs, self.opts);
        // Paragraphs directly in a tight list's items are written bare.
        if tight {
            self.inlines(text, &nodes);
            return;
        }
        self.newline();
        self.out.push_str(if inline::lone_image(text, &nodes).is_some() { "<p class='md-image-only'>" } else { "<p>" });
        self.inlines(text, &nodes);
        self.out.push_str("</p>\n");
    }

    fn code(&mut self, lang: &str, text: &str) {
        self.newline();
        if !self.page.styled {
            if lang.is_empty() {
                self.out.push_str("<pre><code>");
            } else {
                self.out.push_str("<pre><code class='language-");
                self.attr(lang);
                self.out.push_str("' lang='");
                self.attr(lang);
                self.out.push_str("'>");
            }
            self.body(text);
            self.out.push_str("</code></pre>\n");
            return;
        }
        self.out.push_str("<pre class='md-fences'");
        if !lang.is_empty() {
            self.out.push_str(" lang='");
            self.attr(lang);
            self.out.push('\'');
        }
        self.out.push_str("><code>");
        let mut highlight = Highlight::default();
        highlight.update(lang, text);
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                self.out.push('\n');
            }
            let mut at = 0;
            for (range, token) in highlight.runs(i) {
                let (Some(gap), Some(run)) = (line.get(at..range.start), line.get(range.clone())) else { continue };
                self.body(gap);
                if *token == Token::Plain {
                    self.body(run);
                } else {
                    self.out.push_str(&format!("<span class='{}'>", css::token_class(*token)));
                    self.body(run);
                    self.out.push_str("</span>");
                }
                at = range.end;
            }
            self.body(line.get(at..).unwrap_or(""));
        }
        self.out.push_str("</code></pre>\n");
    }

    fn display_math(&mut self, index: usize, tex: &str) {
        self.newline();
        // Without styles, math can stay TeX, for whatever typesets the
        // page it is pasted into.
        if !self.page.styled && self.page.look.math_tex {
            self.out.push_str("<p>$$\n");
            self.body(tex.trim_end());
            self.out.push_str("\n$$</p>\n");
            return;
        }
        let tex = eqnum::with_refs(tex, &self.equations.labels);
        match math::svg(&tex, true, INK) {
            Ok(svg) => {
                self.out.push_str("<div class='md-math-block'>");
                self.out.push_str(&inked(&svg));
                // Numbers at the right, level with their rows, which are
                // taken to be of a height, as the editor places them.
                if let Some(rows) = self.equations.rows.get(&index) {
                    for (i, tag) in rows.iter().enumerate().filter(|(_, t)| !t.is_empty()) {
                        let top = (i as f64 + 0.5) * 100.0 / rows.len() as f64;
                        self.out.push_str(&format!("<span class='md-math-tag' style='top: {top:.2}%'>"));
                        self.body(tag);
                        self.out.push_str("</span>");
                    }
                }
                self.out.push_str("</div>\n");
            }
            Err(e) => {
                self.out.push_str("<p class='md-math-error'>");
                self.body(&e);
                self.out.push_str("</p>\n");
            }
        }
    }

    fn diagram(&mut self, lang: &str, src: &str) {
        let s = &self.page.look.scheme;
        let colors = diagram::Colors { background: s.background, foreground: s.foreground, accent: s.accent, border: s.border };
        self.newline();
        match diagram::svg(lang, src, &colors) {
            Ok(svg) => {
                self.out.push_str("<div class='md-diagram'>");
                self.out.push_str(svg.trim());
                self.out.push_str("</div>\n");
            }
            Err(e) => {
                self.out.push_str("<pre class='md-diagram-error md-math-error'>");
                self.body(&e);
                self.out.push_str("</pre>\n");
            }
        }
    }

    fn table(&mut self, align: &[Align], rows: &[Vec<String>]) {
        self.newline();
        self.out.push_str("<figure class='table-figure'><table>\n");
        for (r, row) in rows.iter().enumerate() {
            let tag = if r == 0 { "th" } else { "td" };
            match r {
                0 => self.out.push_str("<thead>\n"),
                1 => self.out.push_str("<tbody>"),
                _ => {}
            }
            self.out.push_str("<tr>");
            for (col, a) in align.iter().enumerate() {
                self.out.push_str(&format!("<{tag}"));
                self.out.push_str(match a {
                    Align::Left => " style='text-align:left;' >",
                    Align::Center => " style='text-align:center;' >",
                    Align::Right => " style='text-align:right;' >",
                    Align::None => ">",
                });
                // Escaped pipes are plain pipes, even inside code spans.
                match row.get(col).map(|c| c.replace("\\|", "|")).filter(|c| !c.trim().is_empty()) {
                    Some(cell) => self.inline_text(&cell),
                    None => self.out.push_str("&nbsp;"),
                }
                self.out.push_str(&format!("</{tag}>"));
            }
            self.out.push_str("</tr>");
            if r == 0 {
                self.out.push_str("</thead>\n");
            }
        }
        if rows.len() > 1 {
            self.out.push_str("</tbody>\n");
        }
        self.out.push_str("</table></figure>\n");
    }

    fn toc(&mut self) {
        self.newline();
        if !self.page.styled {
            self.out.push_str("<div>[TOC]</div>\n");
            return;
        }
        self.out.push_str("<div class='md-toc'><p class='md-toc-content' role='list'>");
        self.toc = true;
        for h in query::outline(self.doc, self.refs, self.opts) {
            self.out.push_str(&format!("<span role='listitem' class='md-toc-item md-toc-h{}'><a class='md-toc-inner' href='#", h.level));
            let anchor = self.tree.anchor(h.block).unwrap_or("").to_string();
            self.href(&anchor);
            self.out.push_str("'>");
            self.inline_text(&self.doc.blocks[h.block].text);
            self.out.push_str("</a></span>");
        }
        self.toc = false;
        self.out.push_str("</p></div>\n");
    }

    /// The outline sidebar's list: the headings nested by level, each a
    /// link to its heading. Empty with fewer than two headings, as Typora
    /// leaves the sidebar out then.
    fn outline(&mut self) -> String {
        let heads = query::outline(self.doc, self.refs, self.opts);
        if heads.len() < 2 {
            return String::new();
        }
        let page = std::mem::take(&mut self.out);
        self.out.push_str("<ul class='outline-children'>");
        // The open items' levels, and whether each has opened its list.
        let mut open: Vec<(u8, bool)> = Vec::new();
        let close = |out: &mut String, (_, list): (u8, bool)| out.push_str(if list { "</ul></li>" } else { "</li>" });
        for (i, h) in heads.iter().enumerate() {
            while open.last().is_some_and(|&(level, _)| level >= h.level) {
                close(&mut self.out, open.pop().unwrap());
            }
            if let Some(parent) = open.last_mut().filter(|(_, list)| !list) {
                parent.1 = true;
                self.out.push_str("<ul class='outline-children'>");
            }
            let single = heads.get(i + 1).is_none_or(|n| n.level <= h.level);
            self.out.push_str(&format!(
                "<li class='outline-item-wrapper outline-h{}{}'><div class='outline-item'><span class='outline-expander'></span><a class='outline-label' href='#",
                h.level,
                if single { " outline-item-single" } else { "" }
            ));
            let anchor = self.tree.anchor(h.block).unwrap_or("").to_string();
            self.href(&anchor);
            // The heading's text, as the outline panel shows it.
            self.out.push_str(&format!("'>{}</a></div>", html_escape::encode_text(&h.title)));
            open.push((h.level, false));
        }
        while let Some(item) = open.pop() {
            close(&mut self.out, item);
        }
        self.out.push_str("</ul>");
        std::mem::replace(&mut self.out, page)
    }

    /// The footnotes, at the end, in order of first reference, each with a
    /// link back to every reference to it, as Typora's appendFootnote.
    fn footnotes(&mut self) {
        let tree = self.tree;
        if tree.footnotes.is_empty() {
            return;
        }
        self.newline();
        self.out.push_str("<div class='footnotes-area'><hr />\n");
        for f in &tree.footnotes {
            self.out.push_str(&format!("<div class='footnote-line'><span class='md-fn-count'>{n}</span>", n = f.number));
            // A note of one paragraph is written on the line.
            let single = matches!(f.children.as_slice(), [Part::Block { block, .. }] if block.kind == BlockKind::Paragraph);
            self.nodes(&f.children, single);
            for k in 0..self.seen.get(f.number).copied().unwrap_or(0) {
                let id = tree::ref_id(f.number, k);
                self.out.push_str(&format!("<a class='reversefootnote' id='dfref-footnote-{id}' href='#ref-footnote-{id}' title='back to document'>↩</a>"));
            }
            self.newline();
            self.out.push_str("</div>\n");
        }
        self.out.push_str("</div>\n");
    }

    // -- inline -------------------------------------------------------------

    fn inline_text(&mut self, src: &str) {
        let nodes = inline::parse(src, self.refs, self.opts);
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
                Kind::Math if !self.page.styled && self.page.look.math_tex => self.body(&format!("${content}$")),
                Kind::Math => {
                    let tex = eqnum::with_refs(content, &self.equations.labels);
                    match math::svg(&tex, false, INK) {
                        Ok(svg) => {
                            self.out.push_str("<span class='md-math-inline'>");
                            self.out.push_str(&inked(&svg));
                            self.out.push_str("</span>");
                        }
                        Err(_) => {
                            self.out.push_str("<code class='md-math-error'>");
                            self.body(&src[n.range.clone()]);
                            self.out.push_str("</code>");
                        }
                    }
                }
                Kind::Link { dest, title } => {
                    if self.toc {
                        self.inlines(src, &n.children);
                        continue;
                    }
                    self.out.push_str("<a href='");
                    self.href(dest);
                    self.out.push('\'');
                    self.title(title);
                    self.out.push('>');
                    self.inlines(src, &n.children);
                    self.out.push_str("</a>");
                }
                Kind::Image { dest, title } => {
                    if self.toc {
                        continue;
                    }
                    self.out.push_str("<img src='");
                    let src_path = self.image_src(dest);
                    self.href(&src_path);
                    self.out.push_str("' referrerpolicy='no-referrer' alt='");
                    let mut alt = String::new();
                    inline::plain_text(src, &n.children, &mut alt);
                    self.attr(&alt);
                    self.out.push('\'');
                    self.title(title);
                    self.out.push_str(" />");
                }
                Kind::ImgTag(tag) => {
                    if self.toc {
                        continue;
                    }
                    let raw = &src[n.range.clone()];
                    let moved = self.image_src(&tag.src);
                    if moved == tag.src {
                        self.out.push_str(raw);
                    } else {
                        self.out.push_str(&imgtag::with_attr(raw, "src", &moved));
                    }
                }
                Kind::Autolink { dest } => {
                    if self.toc {
                        self.body(content);
                        continue;
                    }
                    self.out.push_str("<a href='");
                    self.href(dest);
                    self.out.push_str("'>");
                    self.body(content);
                    self.out.push_str("</a>");
                }
                Kind::FootnoteRef { label } => match self.tree.footnote_number(label) {
                    Some(_) if self.toc => {}
                    Some(number) => {
                        if self.seen.len() <= number {
                            self.seen.resize(number + 1, 0);
                        }
                        let k = self.seen[number];
                        self.seen[number] += 1;
                        let id = tree::ref_id(number, k);
                        self.out.push_str(&format!("<sup class='md-footnote'><a href='#dfref-footnote-{id}' id='ref-footnote-{id}'>{number}</a></sup>"));
                    }
                    None => self.body(&src[n.range.clone()]),
                },
                Kind::HtmlTag { .. } | Kind::Html | Kind::Comment => self.out.push_str(&src[n.range.clone()]),
                Kind::Tag { .. } => {
                    self.out.push_str(&src[n.range.start..n.content.start]);
                    self.inlines(src, &n.children);
                    self.out.push_str(&src[n.content.end..n.range.end]);
                }
                Kind::Emoji(e) => self.out.push_str(e),
                Kind::Smart(c) => self.out.push(*c),
                Kind::SoftBreak if self.page.look.line_breaks => self.out.push_str("<br/>\n"),
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

    /// Where the page finds image `src`: as written, unless it is a local
    /// file named relative to a folder other than the page's.
    fn image_src(&self, src: &str) -> String {
        let p = self.page;
        let Some(dest) = p.dest else { return src.to_string() };
        let written = src.trim();
        let relative = !written.starts_with('/') && !written.contains("://") && !written.starts_with("data:");
        let same_folder = p.file.and_then(Path::parent) == Some(dest) && p.image_root.is_none();
        if !relative && p.image_root.is_none() || same_folder {
            return src.to_string();
        }
        match image::local_path(written, p.file, p.image_root) {
            Some(path) => image::relative(dest, &path, false),
            None => src.to_string(),
        }
    }

    fn title(&mut self, title: &str) {
        if !title.is_empty() {
            self.out.push_str(" title='");
            self.attr(title);
            self.out.push('\'');
        }
    }

    fn line(&mut self, s: &str) {
        self.newline();
        self.out.push_str(s);
        self.out.push('\n');
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

/// Typeset math in the colour of the text around it.
fn inked(svg: &str) -> String {
    svg.replace(&format!("#{INK}"), "currentColor")
}

/// A code span's text: line breaks become spaces, and one space is stripped
/// from each end when both ends have one.
fn code_text(s: &str) -> String {
    let s = s.replace('\n', " ");
    if s.len() >= 2 && s.starts_with(' ') && s.ends_with(' ') && !s.trim_matches(' ').is_empty() {
        s[1..s.len() - 1].to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eqnum::Numbering;
    use crate::md::parse;
    use crate::palette::Scheme;

    fn look() -> Look {
        Look { scheme: Scheme::paper(), numbering: Numbering::Ams, math_fences: true, diagrams: true, math_tex: true, ..Look::test() }
    }

    fn html(src: &str, styled: bool) -> String {
        let doc = parse(src);
        let refs = inline::refs_of(&doc);
        let look = look();
        let p = Page { title: Some("t"), styled, look: &look, head: "", body: "", file: None, image_root: None, dest: None, sidebar: Sidebar::None };
        body(&doc, &refs, &Options::all(), &p)
    }

    #[test]
    fn follows_the_whitespace_and_math_preferences() {
        let doc = parse("one\ntwo $x$\n");
        let refs = inline::refs_of(&doc);
        let look = Look { line_breaks: true, math_tex: false, indent_first_line: true, ..look() };
        let p = Page { title: Some("t"), styled: false, look: &look, head: "", body: "", file: None, image_root: None, dest: None, sidebar: Sidebar::None };
        let out = body(&doc, &refs, &Options::all(), &p);
        assert!(out.starts_with("<p>one<br/>\ntwo <span class='md-math-inline'><svg"), "{out}");
        let styled = page(&doc, &refs, &Options::all(), &Page { styled: true, ..p });
        assert!(styled.contains("<div id='write' class='first-line-indent'>"));
    }

    #[test]
    fn writes_blocks() {
        assert_eq!(html("# Hi there\n\ntext *em*\n", true), "<h1 id='hi-there'>Hi there</h1>\n<p>text <em>em</em></p>\n");
        assert_eq!(html("- a\n- b\n", true), "<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n");
        assert_eq!(html("3. a\n\n4. b\n", true), "<ol start='3'>\n<li>\n<p>a</p>\n</li>\n<li>\n<p>b</p>\n</li>\n</ol>\n");
        assert_eq!(
            html("- [x] done\n- [ ] not\n", true),
            "<ul class='contains-task-list'>\n<li class='task-list-item'><input type='checkbox' disabled checked />done</li>\n\
             <li class='task-list-item'><input type='checkbox' disabled />not</li>\n</ul>\n"
        );
        assert_eq!(html("> q\n", true), "<blockquote>\n<p>q</p>\n</blockquote>\n");
        assert_eq!(html("---\n", true), "<hr />\n");
    }

    #[test]
    fn writes_alerts() {
        let out = html("> [!WARNING]\n> Careful.\n", true);
        assert!(out.starts_with("<div class='md-alert md-alert-warning'>\n<p class='md-alert-text-container'><span class='md-alert-text md-alert-text-warning'><svg"));
        assert!(out.ends_with("<span>Warning</span></span></p>\n<p>Careful.</p>\n</div>\n"));
        // Without styles, no icon.
        assert!(!html("> [!NOTE]\n> x\n", false).contains("<svg"));
    }

    #[test]
    fn writes_tables() {
        assert_eq!(
            html("| a | b |\n|:-|-:|\n| `x\\|y` | |\n", true),
            "<figure class='table-figure'><table>\n<thead>\n<tr><th style='text-align:left;' >a</th><th style='text-align:right;' >b</th></tr></thead>\n\
             <tbody><tr><td style='text-align:left;' ><code>x|y</code></td><td style='text-align:right;' >&nbsp;</td></tr></tbody>\n</table></figure>\n"
        );
    }

    #[test]
    fn writes_code() {
        assert_eq!(html("```js\nlet a = 1\n```\n", false), "<pre><code class='language-js' lang='js'>let a = 1</code></pre>\n");
        let styled = html("```js\nlet a = \"<\"\n```\n", true);
        assert!(styled.starts_with("<pre class='md-fences' lang='js'><code><span class='tk-keyword'>let</span>"), "{styled}");
        assert!(styled.contains("&lt;"));
        assert_eq!(html("    x < y\n", true), "<pre class='md-fences'><code>x &lt; y</code></pre>\n");
    }

    #[test]
    fn writes_footnotes_at_the_end() {
        let out = html("a[^n] b[^n]\n\n[^n]: The *note*.\n", true);
        assert_eq!(
            out,
            "<p>a<sup class='md-footnote'><a href='#dfref-footnote-1' id='ref-footnote-1'>1</a></sup> b<sup class='md-footnote'><a href='#dfref-footnote-1-1' id='ref-footnote-1-1'>1</a></sup></p>\n\
             <div class='footnotes-area'><hr />\n<div class='footnote-line'><span class='md-fn-count'>1</span>The <em>note</em>.\
             <a class='reversefootnote' id='dfref-footnote-1' href='#ref-footnote-1' title='back to document'>↩</a>\
             <a class='reversefootnote' id='dfref-footnote-1-1' href='#ref-footnote-1-1' title='back to document'>↩</a>\n</div>\n</div>\n"
        );
    }

    #[test]
    fn writes_a_toc_of_links() {
        let out = html("[TOC]\n\n# A [link](x)\n\n## B\n", true);
        assert!(out.starts_with(
            "<div class='md-toc'><p class='md-toc-content' role='list'><span role='listitem' class='md-toc-item md-toc-h1'><a class='md-toc-inner' href='#a-link'>A link</a></span>\
             <span role='listitem' class='md-toc-item md-toc-h2'><a class='md-toc-inner' href='#b'>B</a></span></p></div>\n"
        ));
        assert!(html("[TOC]\n", false).starts_with("<div>[TOC]</div>\n"));
    }

    #[test]
    fn draws_math_and_numbers_it() {
        let out = html("$$\n\\begin{equation}x\\label{a}\\end{equation}\n$$\n\nSee $\\eqref{a}$.\n", true);
        assert!(out.starts_with("<div class='md-math-block'><svg"), "{out}");
        assert!(out.contains("<span class='md-math-tag' style='top: 50.00%'>(1)</span></div>"));
        assert!(out.contains("<span class='md-math-inline'><svg"));
        assert!(out.contains("currentColor") && !out.contains(INK));
        let plain = html("$$\nx^2\n$$\n\nand $y<1$\n", false);
        assert_eq!(plain, "<p>$$\nx^2\n$$</p>\n<p>and $y&lt;1$</p>\n");
    }

    #[test]
    fn moves_relative_images() {
        let doc = parse("![a](img/x.png) ![b](https://e.com/y.png)\n");
        let refs = inline::refs_of(&doc);
        let look = look();
        let file = Path::new("/home/u/docs/note.md");
        let p = Page {
            title: Some("t"),
            styled: true,
            look: &look,
            head: "",
            body: "",
            file: Some(file),
            image_root: None,
            dest: Some(Path::new("/home/u/out")),
            sidebar: Sidebar::None,
        };
        let out = body(&doc, &refs, &Options::all(), &p);
        assert!(out.contains("src='../docs/img/x.png'"), "{out}");
        assert!(out.contains("src='https://e.com/y.png'"));
        let p = Page { dest: Some(Path::new("/home/u/docs")), ..p };
        assert!(body(&doc, &refs, &Options::all(), &p).contains("src='img/x.png'"));
    }

    #[test]
    fn wraps_the_page() {
        let doc = parse("x\n");
        let refs = inline::refs_of(&doc);
        let look = look();
        let p = Page {
            title: Some("a<b"),
            styled: true,
            look: &look,
            head: "<meta name='x'>",
            body: "<script></script>",
            file: None,
            image_root: None,
            dest: None,
            sidebar: Sidebar::None,
        };
        let out = page(&doc, &refs, &Options::all(), &p);
        assert!(out.starts_with("<!doctype html>\n<html>\n<head>\n<meta charset='UTF-8'>"));
        assert!(out.contains("<title>a&lt;b</title>\n<style type='text/css'>\n:root"));
        assert!(out.contains("<meta name='x'>\n</head>\n<body class='typora-export'>\n<div class='typora-export-content'>\n<div id='write'>\n<p>x</p>\n</div>\n</div>\n<script></script>\n</body>\n</html>\n"));
        let plain = page(&doc, &refs, &Options::all(), &Page { styled: false, ..p });
        assert!(!plain.contains("<style") && plain.contains("<body>\n<p>x</p>\n"));
        // Markup with a title of its own replaces the page's.
        let untitled = page(&doc, &refs, &Options::all(), &Page { title: None, head: "<title>Mine</title>", ..p });
        assert!(!untitled.contains("a&lt;b") && untitled.contains("<title>Mine</title>"));
    }

    #[test]
    fn adds_the_outline_sidebar() {
        let doc = parse("# One\n\n## *Two*\n\n### Three\n\n# Four\n");
        let refs = inline::refs_of(&doc);
        let look = look();
        let p = Page { title: None, styled: true, look: &look, head: "", body: "", file: None, image_root: None, dest: None, sidebar: Sidebar::Collapsible };
        let out = page(&doc, &refs, &Options::all(), &p);
        assert!(out.contains("<body class='typora-export typora-export-show-outline typora-export-collapse-outline'>"), "{out}");
        let item = |level: u8, single: bool, id: &str, text: &str| {
            format!(
                "<li class='outline-item-wrapper outline-h{level}{}'><div class='outline-item'><span class='outline-expander'></span>\
                 <a class='outline-label' href='#{id}'>{text}</a></div>",
                if single { " outline-item-single" } else { "" }
            )
        };
        let list = format!(
            "<div class='typora-export-sidebar'><div class='outline-content'><ul class='outline-children'>{}<ul class='outline-children'>{}\
             <ul class='outline-children'>{}</li></ul></li></ul></li>{}</li></ul></div></div>",
            item(1, false, "one", "One"),
            item(2, false, "two", "Two"),
            item(3, true, "three", "Three"),
            item(1, true, "four", "Four")
        );
        assert!(out.contains(&list), "{out}");
        assert!(out.contains(".typora-export-sidebar") && out.contains("outline-item-open"));
        // Flat, or none for one heading or without styles.
        let flat = page(&doc, &refs, &Options::all(), &Page { sidebar: Sidebar::Flat, ..p });
        assert!(flat.contains("typora-export-show-outline typora-export-no-collapse-outline"));
        let one = parse("# One\n");
        assert!(!page(&one, &refs, &Options::all(), &p).contains("typora-export-sidebar"));
        assert!(!page(&doc, &refs, &Options::all(), &Page { styled: false, ..p }).contains("typora-export-sidebar"));
    }
}

/// Writes `$TYPO_SAMPLE` as styled pages in the current theme and on paper,
/// for a look at them: `TYPO_SAMPLE=doc.md cargo test sample_pages -- --ignored`.
#[cfg(test)]
#[test]
#[ignore]
fn sample_pages() {
    use crate::palette::{Palette, Scheme, current_dir};
    let path = std::path::PathBuf::from(std::env::var("TYPO_SAMPLE").unwrap());
    let doc = crate::md::parse(&std::fs::read_to_string(&path).unwrap());
    let refs = inline::refs_of(&doc);
    let dest = path.parent().unwrap();
    for (name, scheme) in [("theme", Scheme::new(Palette::load(&current_dir()).unwrap())), ("paper", Scheme::paper())] {
        let look = Look { scheme, numbering: eqnum::Numbering::Ams, math_fences: true, diagrams: true, ..Look::test() };
        let p = Page {
            title: Some("sample"),
            styled: true,
            look: &look,
            head: "",
            body: "",
            file: Some(&path),
            image_root: None,
            dest: Some(dest),
            sidebar: Sidebar::Collapsible,
        };
        std::fs::write(dest.join(format!("{name}.html")), page(&doc, &refs, &Options::all(), &p)).unwrap();
    }
}
