//! The document typeset for paper, as Typora's PDF and image exports lay
//! it out: the tree written as Typst markup in typo's look (the exported
//! page's sizes, scaled to the print size), typeset with the system's fonts
//! and written out by typst-pdf, or drawn by typst-render for an image.
//! Math is typeset by the same mitex functions the editor uses, diagrams
//! are SVG, and code is coloured run by run as the editor colours it.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration, Smart};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::{LazyHash, Scalar};
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;

use super::tree::{self, Node as Part, Tree};
use super::{Equations, Look, fence_lang};
use crate::code::{Highlight, Token};
use crate::md::inline::{self, Kind, Node, Options, Refs};
use crate::md::query::{self, ALERTS};
use crate::md::{Align, Block, BlockKind, Document, ListMarker, from_html};
use crate::palette::{ALERT_COLORS, Rgb};
use crate::{diagram, eqnum, image, math};

/// Typora's PDF settings (Preferences → Export → PDF).
#[derive(Clone, Debug)]
pub struct PdfSettings {
    /// The page's width and height in millimetres.
    pub page: (f64, f64),
    /// Margins in millimetres: top, right, bottom, left.
    pub margins: [f64; 4],
    /// What the top and bottom of every page say, their variables filled
    /// in but for the page's number and the number of pages, which are
    /// `PAGE_NO` and `PAGE_COUNT`.
    pub header: String,
    pub footer: String,
    /// The PDF's metadata.
    pub author: String,
    pub subject: String,
    pub keywords: Vec<String>,
    /// What made the PDF; Typst's name when empty.
    pub creator: String,
    /// Each top-level heading starts a page.
    pub break_on_h1: bool,
}

/// Where a header or footer shows the page's number, and the number of
/// pages.
pub const PAGE_NO: char = '\u{E001}';
pub const PAGE_COUNT: char = '\u{E002}';

impl Default for PdfSettings {
    fn default() -> Self {
        PdfSettings {
            page: (210.0, 297.0),
            margins: [16.0; 4],
            header: String::new(),
            footer: String::new(),
            author: String::new(),
            subject: String::new(),
            keywords: Vec::new(),
            creator: String::new(),
            break_on_h1: false,
        }
    }
}

/// Pages for a PDF, or one long page for an image.
pub enum Sheet<'a> {
    Pages(&'a PdfSettings),
    /// Typora's image export: a page `width` pixels wide with text of
    /// `font` pixels, as tall as the document.
    Image { width: f64, font: f64 },
}

pub struct Paper<'a> {
    pub title: &'a str,
    pub look: &'a Look,
    /// The family the desktop's monospace font resolves to.
    pub mono_font: &'a str,
    pub file: Option<&'a Path>,
    pub image_root: Option<&'a Path>,
    pub sheet: Sheet<'a>,
}

/// Typora prints at 13px, against 16px on screen.
const PRINT_FONT: f64 = 13.0;

/// Typora gives up on an image taller than this, in pixels.
pub const MAX_IMAGE_HEIGHT: f64 = 38400.0;

/// The document as a PDF.
pub fn pdf(doc: &Document, refs: &Refs, opts: &Options, paper: &Paper) -> Result<Vec<u8>, String> {
    let document = typeset(doc, refs, opts, paper)?;
    let mut options = typst_pdf::PdfOptions::default();
    if let Sheet::Pages(set) = &paper.sheet
        && !set.creator.is_empty()
    {
        options.creator = Smart::Custom(Some(set.creator.clone()));
    }
    typst_pdf::pdf(&document, &options).map_err(|e| first_error(&e))
}

/// The most pixels Image Quality ▸ Auto draws an image with before it
/// lowers the scale.
const AUTO_PIXELS: f64 = 40e6;

/// The document drawn as one image `scale` pixels to the page's pixel,
/// or at most that when `auto` and the image is large: its width, height
/// and premultiplied RGBA pixels.
pub fn image(doc: &Document, refs: &Refs, opts: &Options, paper: &Paper, scale: f64, auto: bool) -> Result<(u32, u32, Vec<u8>), String> {
    let document = typeset(doc, refs, opts, paper)?;
    let page = document.pages().first().ok_or("Nothing to export")?;
    let (w, h) = (page.frame.width().to_pt() / 0.75, page.frame.height().to_pt() / 0.75);
    if h > MAX_IMAGE_HEIGHT {
        return Err("Generated Image too Large".into());
    }
    let scale = if auto { scale.min((AUTO_PIXELS / (w * h).max(1.0)).sqrt()).max(1.0) } else { scale };
    let pixmap = typst_render::render(page, &typst_render::RenderOptions { pixel_per_pt: Scalar::new(scale / 0.75), render_bleed: false });
    Ok((pixmap.width(), pixmap.height(), pixmap.take()))
}

/// The Typst markup for the document, and the files it reads.
pub fn markup(doc: &Document, refs: &Refs, opts: &Options, paper: &Paper) -> (String, HashMap<String, Bytes>) {
    let tree = tree::build(doc, refs, opts);
    let equations = paper.look.equations(doc);
    let mut w = Writer {
        doc,
        refs,
        opts,
        paper,
        tree: &tree,
        equations: &equations,
        files: HashMap::new(),
        labelled: Vec::new(),
        list_level: 0,
        toc: false,
        out: prelude(paper),
    };
    w.nodes(&tree.body, false);
    w.footnotes();
    (w.out, w.files)
}

fn typeset(doc: &Document, refs: &Refs, opts: &Options, paper: &Paper) -> Result<PagedDocument, String> {
    let (main, files) = markup(doc, refs, opts, paper);
    let world = Job { main: Source::new(id("/main.typ"), main), files };
    let result = typst::compile::<PagedDocument>(&world).output;
    typst::comemo::evict(30);
    result.map_err(|e| first_error(&e))
}

fn first_error(errors: &[typst::diag::SourceDiagnostic]) -> String {
    errors.first().map(|e| e.message.to_string()).unwrap_or_else(|| "Export failed".into())
}

fn color(c: Rgb) -> String {
    format!("rgb(\"{}\")", c.hex())
}

/// A Typst string literal.
fn lit(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The definitions every document starts with: the page, the text, and
/// the pieces blocks are drawn with. `px` is a pixel of the exported page
/// at the size it is printed, so the HTML stylesheet's sizes carry over.
fn prelude(paper: &Paper) -> String {
    let s = &paper.look.scheme;
    let (font, page) = match &paper.sheet {
        Sheet::Pages(set) => {
            let [top, right, bottom, left] = set.margins;
            let margin = format!("(top: {top}mm, right: {right}mm, bottom: {bottom}mm, left: {left}mm)");
            let header = template(&set.header, top);
            let footer = template(&set.footer, bottom);
            let keywords: Vec<String> = set.keywords.iter().map(|k| lit(k)).collect();
            (
                PRINT_FONT,
                format!(
                    "#set page(width: {}mm, height: {}mm, margin: {margin}, fill: {}, header: {header}, footer: {footer}, \
                     header-ascent: 0%, footer-descent: 0%)\n\
                     #set document(title: {}, author: {}, description: {}, keywords: ({}))\n",
                    set.page.0,
                    set.page.1,
                    color(s.background),
                    lit(paper.title),
                    if set.author.is_empty() { "()".to_string() } else { lit(&set.author) },
                    if set.subject.is_empty() { "none".to_string() } else { lit(&set.subject) },
                    keywords.iter().map(|k| format!("{k},")).collect::<String>(),
                ),
            )
        }
        Sheet::Image { width, font } => (
            *font,
            format!(
                "#set page(width: {}pt, height: auto, margin: {}pt, fill: {})\n#set document(title: {})\n",
                width * 0.75,
                font * 0.75,
                color(s.background),
                lit(paper.title)
            ),
        ),
    };
    let alerts: Vec<String> = ALERT_COLORS.iter().map(|k| color(s.palette.get(k))).collect();
    let heads: Vec<String> = s.headings.iter().map(|c| color(*c)).collect();
    format!(
        r##"#import "/specs/mod.typ": mitex-scope
#let px = {px}pt
#let dot = 0.75pt
#let fg = {fg}
#let muted = {muted}
#let border = {border}
#let accent = {accent}
#let codebg = {codebg}
#let mono = ({mono}, "DejaVu Sans Mono")
#let alerts = ({alerts},)
{page}#set text(font: ("Open Sans", "Clear Sans", "Helvetica Neue", "Noto Sans", "Liberation Sans", "DejaVu Sans"), size: 16 * px, fill: fg, top-edge: 0.9em, bottom-edge: -0.25em, hyphenate: false)
#set par(leading: 0.45em, spacing: 12.8 * px, justify: false)
#show math.equation: set text(font: "New Computer Modern Math")
#show link: set text(fill: {link})
#show heading: it => {{
  let i = it.level - 1
  let size = (2.25, 1.75, 1.5, 1.25, 1, 1).at(i) * 16 * px
  let ruled = it.level <= 2
  block(above: 16 * px, below: 16 * px, width: 100%, sticky: true,
    stroke: if ruled {{ (bottom: 1 * px + border) }} else {{ none }},
    inset: if ruled {{ (bottom: 0.3 * size) }} else {{ 0pt }}, {{
      set par(leading: if ruled {{ 0.05em }} else {{ 0.25em }})
      text(size: size, weight: "bold", fill: ({heads}).at(i), it.body)
    }})
}}
#let para(m, body) = block(above: m * px, below: m * px, width: 100%, body)
#let item(marker, gap, body) = block(above: gap * px, below: gap * px, width: 100%,
  grid(columns: (30 * px, 1fr), align(right, box(inset: (right: 8 * px), marker)), body))
#let tick = curve(stroke: (paint: {accent_text}, thickness: 1.75 * px, cap: "round", join: "round"),
  curve.move((3 * px, 7.5 * px)), curve.line((5.7 * px, 10 * px)), curve.line((11 * px, 3.9 * px)))
#let task(done) = pad(top: 0.575em - 7 * px, box(width: 14 * px, height: 14 * px, radius: 3 * px,
  fill: if done {{ accent }} else {{ none }}, stroke: 1.2 * px + if done {{ accent }} else {{ muted }}, if done {{ place(top + left, tick) }}))
#let pic(path, w, alt) = layout(size => image(path, alt: alt, width: calc.min(w * dot, size.width)))
#let code(body) = box(fill: codebg, inset: (x: 0.2em), outset: (y: 0.2em), radius: 3 * px, text(font: mono, size: 0.9em, fill: {code}, body))
#let kbd(body) = box(fill: codebg, stroke: 1 * px + border, inset: (x: 0.3em), outset: (y: 0.15em), radius: 3 * px, text(font: mono, size: 0.9em, body))
#let fnref(n, target) = super(link(target, box(fill: codebg, inset: (x: 3 * px), outset: (y: 0.1em), radius: 3 * px, str(n))))
#let fence(body) = block(above: 15 * px, below: 15 * px, width: 100%, fill: codebg, stroke: 1 * px + {fence_border},
  radius: 3 * px, inset: (x: 12 * px, y: 8 * px), {{
    set text(font: mono, size: 0.9em, fill: fg)
    set par(leading: 0.3em, spacing: 0.3em)
    body
  }})
#let formula(eq, tags) = context {{
  let body = text(size: 1.18em, eq)
  let h = measure(body).height
  block(width: 100%, {{
    align(center, body)
    for (i, t) in tags.enumerate() {{
      if t != "" {{ place(top + right, dy: (i + 0.5) / tags.len() * h - 0.5em, text(size: 1.18em, t)) }}
    }}
  }})
}}
#let quote(color, inset, body) = block(above: 12.8 * px, below: 12.8 * px, width: 100%,
  stroke: (left: 4 * px + color), inset: (left: inset * px), body)
#let rule = block(above: 16 * px, below: 16 * px, line(length: 100%, stroke: 2 * px + border))
"##,
        px = 0.75 * font / 16.0,
        fg = color(s.foreground),
        muted = color(s.muted),
        border = color(s.border),
        accent = color(s.accent),
        accent_text = color(s.accent_text),
        codebg = color(s.code_background),
        code = color(s.code),
        link = color(s.link),
        mono = lit(paper.mono_font),
        alerts = alerts.join(", "),
        heads = heads.join(", "),
        fence_border = format!("{}.transparentize(40%)", color(s.border)),
    )
}

/// A header or footer as Typst: 12px text centred in a margin `depth`
/// millimetres deep, as Typora prints them, with the page's number and
/// the count where `PAGE_NO` and `PAGE_COUNT` are.
fn template(text: &str, depth: f64) -> String {
    if text.trim().is_empty() {
        return "none".into();
    }
    let mut body = String::new();
    let mut run = String::new();
    for c in text.chars() {
        if c == PAGE_NO || c == PAGE_COUNT {
            body.push_str(&format!("#{};", lit(&std::mem::take(&mut run))));
            body.push_str(if c == PAGE_NO { "#counter(page).display();" } else { "#str(counter(page).final().first());" });
        } else {
            run.push(c);
        }
    }
    body.push_str(&format!("#{};", lit(&run)));
    format!("context block(width: 100%, height: {depth}mm, align(center + horizon, text(size: 9pt, fill: fg)[{body}]))")
}

struct Writer<'a> {
    doc: &'a Document,
    refs: &'a Refs,
    opts: &'a Options,
    paper: &'a Paper<'a>,
    tree: &'a Tree<'a>,
    equations: &'a Equations,
    files: HashMap<String, Bytes>,
    /// How many references to each footnote have been labelled, by number.
    labelled: Vec<usize>,
    /// How many list items the block being written is in.
    list_level: usize,
    toc: bool,
    out: String,
}

impl Writer<'_> {
    fn nodes(&mut self, nodes: &[Part], tight: bool) {
        for n in nodes {
            match n {
                Part::Block { index, block, skip } => self.block(*index, block, *skip, tight),
                Part::Quote { alert: None, children } => {
                    self.out.push_str("#quote(border, 15)[#set text(fill: muted)\n");
                    self.nodes(children, false);
                    self.out.push_str("]\n");
                }
                Part::Quote { alert: Some(kind), children } => self.alert(kind, children),
                Part::List(list) => self.list(list, tight),
            }
        }
    }

    fn alert(&mut self, kind: &str, children: &[Part]) {
        let i = ALERTS.iter().position(|a| *a == kind).unwrap_or(0);
        let c = self.paper.look.scheme.palette.get(ALERT_COLORS[i]).hex();
        let icon = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='{}' fill='none' stroke='{c}' stroke-width='1.3' \
             stroke-linecap='round' stroke-linejoin='round'/></svg>",
            super::html::ALERT_ICONS[i]
        );
        let mut title = kind.to_string();
        title[..1].make_ascii_uppercase();
        self.out.push_str(&format!(
            "#quote(alerts.at({i}), 16)[#block(above: 0pt, below: 12.8 * px, text(size: 0.9em, weight: \"bold\", fill: alerts.at({i}))[#box(baseline: 0.2em, image(bytes({}), width: 16 * px));#h(0.4em)#{};])\n",
            lit(&icon),
            lit(&title)
        ));
        self.nodes(children, false);
        self.out.push_str("]\n");
    }

    fn list(&mut self, list: &tree::List, tight: bool) {
        // A list in a tight list's item sits right under the item's text.
        let gap = if tight { 0.0 } else { 12.8 };
        let item_gap = if list.loose { 12.8 } else { 0.0 };
        self.out.push_str(&format!("#block(above: {gap} * px, below: {gap} * px, width: 100%)[\n"));
        for (n, item) in list.items.iter().enumerate() {
            let marker = match (item.task, &list.marker) {
                (Some(done), _) => format!("task({done})"),
                (None, ListMarker::Bullet(_)) => format!("text(size: 1.1em, top-edge: 0.9em / 1.1)[{}]", ["•", "◦", "▪"][self.list_level % 3]),
                (None, m) => {
                    // Numbers go on from the list's first.
                    let m = match m {
                        ListMarker::Ordered { delim, same_numbers, .. } => {
                            ListMarker::Ordered { start: list.start.unwrap_or(1), delim: *delim, same_numbers: *same_numbers }
                        }
                        m => m.clone(),
                    };
                    lit(&m.text(n))
                }
            };
            // Indent first line moves list items in by as much.
            let pad = self.paper.look.indent_first_line;
            if pad {
                self.out.push_str("#pad(left: 2em)[");
            }
            self.out.push_str(&format!("#item({marker}, {item_gap})[\n"));
            self.list_level += 1;
            self.nodes(&item.children, !list.loose);
            self.list_level -= 1;
            self.out.push_str("]\n");
            if pad {
                self.out.push_str("]\n");
            }
        }
        self.out.push_str("]\n");
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
                if *level == 1 && index > 0 && matches!(self.paper.sheet, Sheet::Pages(s) if s.break_on_h1) {
                    self.out.push_str("#pagebreak(weak: true)\n");
                }
                self.out.push_str(&format!("#heading(level: {level})["));
                self.inline_text(&b.text);
                self.out.push(']');
                if let Some(anchor) = self.tree.anchor(index) {
                    self.out.push_str(&format!("#label({})", lit(anchor)));
                }
                self.out.push('\n');
            }
            BlockKind::CodeFence { info, .. } => {
                let lang = fence_lang(info).to_lowercase();
                if self.paper.look.is_math(&b.kind) {
                    self.display_math(index, &b.text);
                } else if self.paper.look.diagrams && diagram::LANGUAGES.contains(&lang.as_str()) {
                    self.diagram(&lang, &b.text);
                } else {
                    self.code(&lang, &b.text);
                }
            }
            BlockKind::IndentedCode => self.code("", &b.text),
            BlockKind::Math { .. } => self.display_math(index, &b.text),
            BlockKind::Html => self.html(&b.text),
            BlockKind::Table { align, rows } => self.table(align, rows),
            BlockKind::ThematicBreak { .. } => self.out.push_str("#rule\n"),
            BlockKind::Toc => self.toc(),
            BlockKind::FrontMatter => {}
        }
    }

    fn paragraph(&mut self, text: &str, tight: bool) {
        if text.trim().is_empty() {
            return;
        }
        let nodes = inline::parse(text, self.refs, self.opts);
        self.out.push_str(&format!("#para({})[", if tight { "0" } else { "12.8" }));
        if inline::lone_image(text, &nodes).is_some() {
            self.out.push_str("#align(center)[");
            self.inlines(text, &nodes);
            self.out.push(']');
        } else {
            // Indent first line: paragraphs outside list items.
            if self.paper.look.indent_first_line && self.list_level == 0 {
                self.out.push_str("#h(2em)");
            }
            self.inlines(text, &nodes);
        }
        self.out.push_str("]\n");
    }

    fn code(&mut self, lang: &str, text: &str) {
        let s = &self.paper.look.scheme;
        let mut highlight = Highlight::default();
        highlight.update(lang, text);
        self.out.push_str("#fence[");
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                self.out.push_str("#linebreak();");
            }
            let mut runs: Vec<(&str, Token)> = Vec::new();
            let mut at = 0;
            for (range, token) in highlight.runs(i) {
                let (Some(gap), Some(run)) = (line.get(at..range.start), line.get(range.clone())) else { continue };
                runs.extend([(gap, Token::Plain), (run, *token)]);
                at = range.end;
            }
            runs.push((line.get(at..).unwrap_or(""), Token::Plain));
            // Spaces at the start of a line hold their place; tabs are
            // four of them.
            let mut lead = true;
            for (run, token) in runs {
                let mut shown = String::new();
                for c in run.chars() {
                    match c {
                        '\t' => shown.push_str(if lead { "\u{a0}\u{a0}\u{a0}\u{a0}" } else { "    " }),
                        ' ' if lead => shown.push('\u{a0}'),
                        c => {
                            lead = false;
                            shown.push(c);
                        }
                    }
                }
                if shown.is_empty() {
                    continue;
                }
                if token == Token::Plain {
                    self.out.push_str(&format!("#{};", lit(&shown)));
                } else {
                    let (key, italic) = token.style();
                    let style = if italic { ", style: \"italic\"" } else { "" };
                    self.out.push_str(&format!("#text(fill: {}{style}, {});", color(s.palette.get(key)), lit(&shown)));
                }
            }
            if line.is_empty() {
                self.out.push_str("#\"\u{a0}\";");
            }
        }
        self.out.push_str("]\n");
    }

    /// An equation, typeset natively, or what is wrong with it.
    fn equation(&self, tex: &str, display: bool) -> Result<String, String> {
        let tex = eqnum::with_refs(tex, &self.equations.labels);
        math::check(&tex, display)?;
        let formula = math::convert(&tex)?;
        Ok(format!("math.equation(block: {display}, numbering: none, eval({}, scope: mitex-scope))", lit(&format!("${formula}$"))))
    }

    fn display_math(&mut self, index: usize, tex: &str) {
        match self.equation(tex, true) {
            Ok(eq) => {
                let tags: Vec<String> = self.equations.rows.get(&index).map_or(Vec::new(), |r| r.iter().map(|t| lit(t)).collect());
                self.out.push_str(&format!("#para(12.8)[#formula({eq}, ({}))]\n", if tags.is_empty() { String::new() } else { tags.join(", ") + "," }));
            }
            Err(e) => self.error(&e),
        }
    }

    fn error(&mut self, message: &str) {
        let error = color(self.paper.look.scheme.error);
        self.out.push_str(&format!("#para(12.8)[#text(fill: {error}, font: mono, size: 0.9em, {})]\n", lit(message)));
    }

    fn diagram(&mut self, lang: &str, src: &str) {
        let s = &self.paper.look.scheme;
        let colors = diagram::Colors { background: s.background, foreground: s.foreground, accent: s.accent, border: s.border };
        match diagram::svg(lang, src, &colors) {
            Ok(svg) => {
                // Diagram text is page text: it scales with the font.
                let font = match self.paper.sheet {
                    Sheet::Pages(_) => PRINT_FONT,
                    Sheet::Image { font, .. } => font,
                };
                let width = svg_width(&svg).unwrap_or(600.0) * font / 16.0;
                let path = self.file("svg", Bytes::from_string(svg));
                self.out.push_str(&format!("#block(above: 15 * px, below: 15 * px, width: 100%, align(center, pic({}, {width}, \"\")))\n", lit(&path)));
            }
            Err(e) => self.error(&e),
        }
    }

    /// Serve `data` to the document; returns its path there.
    fn file(&mut self, ext: &str, data: Bytes) -> String {
        let path = format!("/files/{}.{ext}", self.files.len());
        self.files.insert(path.clone(), data);
        path
    }

    /// An HTML block: what Typora would show of it, through markdown.
    fn html(&mut self, text: &str) {
        let md = from_html::to_markdown(text, self.opts);
        if md.trim().is_empty() {
            return;
        }
        let sub = crate::md::parse(&md);
        let refs = inline::refs_of(&sub);
        let subtree = tree::build(&sub, &refs, self.opts);
        let empty = Equations::default();
        let mut w = Writer {
            doc: &sub,
            refs: &refs,
            opts: self.opts,
            paper: self.paper,
            tree: &subtree,
            equations: &empty,
            files: std::mem::take(&mut self.files),
            labelled: Vec::new(),
            list_level: self.list_level,
            toc: false,
            out: String::new(),
        };
        w.nodes(&subtree.body, false);
        self.files = w.files;
        self.out.push_str(&w.out);
    }

    fn table(&mut self, align: &[Align], rows: &[Vec<String>]) {
        let aligns: Vec<&str> = align
            .iter()
            .map(|a| match a {
                Align::Center => "center",
                Align::Right => "right",
                _ => "left",
            })
            .collect();
        self.out.push_str(&format!(
            "#block(above: 19.2 * px, below: 19.2 * px, table(columns: {}, align: ({},), stroke: 1 * px + border, inset: (x: 13 * px, y: 6 * px), \
             fill: (x, y) => if y == 0 or calc.even(y) {{ codebg }},\n",
            align.len(),
            aligns.join(", ")
        ));
        for (r, row) in rows.iter().enumerate() {
            self.out.push_str(if r == 0 { "table.header(" } else { "" });
            for col in 0..align.len() {
                self.out.push_str(if r == 0 { "strong[" } else { "[" });
                if let Some(cell) = row.get(col) {
                    self.inline_text(&cell.replace("\\|", "|"));
                }
                self.out.push_str("], ");
            }
            self.out.push_str(if r == 0 { "),\n" } else { "\n" });
        }
        self.out.push_str("))\n");
    }

    fn toc(&mut self) {
        let headings = query::outline(self.doc, self.refs, self.opts);
        if headings.is_empty() {
            return;
        }
        self.out.push_str("#block(above: 20 * px, below: 20 * px, width: 100%)[#set text(size: 0.9em)\n");
        self.toc = true;
        for h in headings {
            let indent = [0.0, 2.0, 3.5, 5.0, 6.5, 8.0][(h.level as usize).clamp(1, 6) - 1];
            let Some(anchor) = self.tree.anchor(h.block) else { continue };
            self.out.push_str(&format!("#block(above: 0pt, below: 0pt, inset: (left: {indent}em, y: 0.15em))[#link(label({}))[", lit(anchor)));
            if h.level == 1 {
                self.out.push_str("#strong[");
            }
            self.inline_text(&self.doc.blocks[h.block].text);
            if h.level == 1 {
                self.out.push(']');
            }
            self.out.push_str("]]\n");
        }
        self.toc = false;
        self.out.push_str("]\n");
    }

    /// The footnotes at the end, as the HTML export has them.
    fn footnotes(&mut self) {
        let tree = self.tree;
        if tree.footnotes.is_empty() {
            return;
        }
        self.out.push_str("#block(above: 2em, width: 100%)[#set text(size: 0.9em)\n#rule\n");
        for f in &tree.footnotes {
            self.out.push_str(&format!("#block(above: 0.4em, below: 0.4em, width: 100%)[#{};#h(0.3em)", lit(&format!("{}.", f.number))));
            let single = matches!(f.children.as_slice(), [Part::Block { block, .. }] if block.kind == BlockKind::Paragraph);
            if let [Part::Block { block, skip, .. }] = f.children.as_slice()
                && single
            {
                self.inline_text(&block.text[*skip..]);
            } else {
                self.out.push('\n');
                self.nodes(&f.children, false);
            }
            for k in 0..self.labelled.get(f.number).copied().unwrap_or(0) {
                self.out.push_str(&format!("#h(0.3em)#link(label(\"fnref-{}\"), text(font: mono, \"↩\"));", tree::ref_id(f.number, k)));
            }
            self.out.push_str(&format!("]#label(\"fn-{}\")\n", f.number));
        }
        self.out.push_str("]\n");
    }

    // -- inline -------------------------------------------------------------

    fn inline_text(&mut self, src: &str) {
        let nodes = inline::parse(src, self.refs, self.opts);
        self.inlines(src, &nodes);
    }

    fn text(&mut self, s: &str) {
        if !s.is_empty() {
            self.out.push_str(&format!("#{};", lit(s)));
        }
    }

    fn wrap(&mut self, open: &str, src: &str, children: &[Node]) {
        self.out.push_str(open);
        self.out.push('[');
        self.inlines(src, children);
        self.out.push(']');
    }

    fn inlines(&mut self, src: &str, nodes: &[Node]) {
        for n in nodes {
            let content = &src[n.content.clone()];
            match &n.kind {
                Kind::Text | Kind::Escape => self.text(content),
                Kind::Entity(s) => self.text(s),
                Kind::Code => self.out.push_str(&format!("#code({});", lit(&code_text(content)))),
                Kind::Emph => self.wrap("#emph", src, &n.children),
                Kind::Strong => self.wrap("#strong", src, &n.children),
                Kind::Strike => self.wrap("#strike", src, &n.children),
                Kind::Highlight => self.wrap(&format!("#highlight(fill: {})", color(self.paper.look.scheme.mark)), src, &n.children),
                Kind::Sup => self.wrap("#super", src, &n.children),
                Kind::Sub => self.wrap("#sub", src, &n.children),
                Kind::Math => match self.equation(content, false) {
                    Ok(eq) => self.out.push_str(&format!("#box(text(size: 1.18em, {eq}));")),
                    Err(_) => {
                        let error = color(self.paper.look.scheme.error);
                        self.out.push_str(&format!("#text(fill: {error}, font: mono, size: 0.9em, {});", lit(&src[n.range.clone()])));
                    }
                },
                Kind::Link { dest, .. } => self.link(dest, src, &n.children),
                Kind::Autolink { dest } => {
                    if self.toc {
                        self.text(content);
                    } else {
                        self.out.push_str(&format!("#link({}, {});", lit(dest), lit(content)));
                    }
                }
                Kind::Image { dest, .. } => {
                    let mut alt = String::new();
                    inline::plain_text(src, &n.children, &mut alt);
                    self.picture(dest, &alt, None);
                }
                Kind::ImgTag(tag) => {
                    let width = (!tag.width.is_empty()).then_some(tag.width.as_str()).or((!tag.zoom.is_empty()).then_some(tag.zoom.as_str()));
                    let scale = width.filter(|_| tag.width.is_empty()).and_then(percent);
                    self.picture(&tag.src, &tag.alt, Some((width.filter(|_| !tag.width.is_empty()), scale)));
                }
                Kind::FootnoteRef { label } => match self.tree.footnote_number(label) {
                    Some(_) if self.toc => {}
                    Some(number) => {
                        self.out.push_str(&format!("#fnref({number}, label(\"fn-{number}\"))"));
                        if self.labelled.len() <= number {
                            self.labelled.resize(number + 1, 0);
                        }
                        let k = self.labelled[number];
                        self.labelled[number] += 1;
                        self.out.push_str(&format!("#label(\"fnref-{}\")", tree::ref_id(number, k)));
                    }
                    None => self.text(&src[n.range.clone()]),
                },
                Kind::HtmlTag { name, .. } if name.eq_ignore_ascii_case("br") => self.out.push_str("#linebreak();"),
                Kind::HtmlTag { .. } | Kind::Html | Kind::Comment => {}
                Kind::Tag { name, color: fill, background } => self.tag(name, fill.as_deref(), background.as_deref(), src, &n.children),
                Kind::Emoji(e) => self.text(e),
                Kind::Smart(c) => self.text(&c.to_string()),
                Kind::SoftBreak if self.paper.look.line_breaks => self.out.push_str("#linebreak();"),
                Kind::SoftBreak => self.text(" "),
                Kind::HardBreak => self.out.push_str("#linebreak();"),
            }
        }
    }

    fn link(&mut self, dest: &str, src: &str, children: &[Node]) {
        if self.toc {
            self.inlines(src, children);
            return;
        }
        let target = match dest.strip_prefix('#') {
            // Only headings have anchors here; a link to anything else is
            // its text.
            Some(anchor) => match self.tree.anchors.values().find(|a| a.eq_ignore_ascii_case(anchor)) {
                Some(a) => format!("label({})", lit(a)),
                None => return self.inlines(src, children),
            },
            None => lit(dest),
        };
        self.wrap(&format!("#link({target})"), src, children);
    }

    /// An inline HTML element Typora draws: its look, where Typst has one.
    fn tag(&mut self, name: &str, fill: Option<&str>, background: Option<&str>, src: &str, children: &[Node]) {
        let mut open = Vec::new();
        match name.to_ascii_lowercase().as_str() {
            "u" | "ins" => open.push("#underline".to_string()),
            "s" | "del" | "strike" => open.push("#strike".to_string()),
            "b" | "strong" => open.push("#strong".to_string()),
            "i" | "em" | "cite" => open.push("#emph".to_string()),
            "mark" => open.push(format!("#highlight(fill: {})", color(self.paper.look.scheme.mark))),
            "sup" => open.push("#super".to_string()),
            "sub" => open.push("#sub".to_string()),
            "small" => open.push("#text(size: 0.8em)".to_string()),
            "kbd" => open.push("#kbd".to_string()),
            "code" | "tt" | "samp" => open.push("#code".to_string()),
            _ => {}
        }
        if let Some(c) = fill.and_then(css_color) {
            open.push(format!("#text(fill: {})", color(c)));
        }
        if let Some(c) = background.and_then(css_color) {
            open.push(format!("#highlight(fill: {})", color(c)));
        }
        for o in &open {
            self.out.push_str(o);
            self.out.push('[');
        }
        self.inlines(src, children);
        for _ in &open {
            self.out.push(']');
        }
    }

    /// An image: local ones are read and drawn at their size in pixels (or
    /// `size`, an `<img>`'s width and zoom), at most as wide as the page;
    /// others are their alt text, which is all a page without a network
    /// can show of them.
    fn picture(&mut self, src: &str, alt: &str, size: Option<(Option<&str>, Option<f64>)>) {
        if self.toc {
            return;
        }
        let p = self.paper;
        let path = image::local_path(src, p.file, p.image_root);
        let data = path.as_deref().and_then(|path| std::fs::read(path).ok());
        let ext = path.as_deref().and_then(Path::extension).map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let natural = data.as_deref().and_then(|d| match ext.as_str() {
            "svg" => svg_width(&String::from_utf8_lossy(d)).map(|w| (w, 0.0)),
            _ => imagesize::blob_size(d).ok().map(|s| (s.width as f64, s.height as f64)),
        });
        let (Some(data), Some((width, _))) = (data, natural) else {
            self.text(&format!("[{}]", if alt.is_empty() { src } else { alt }));
            return;
        };
        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp") {
            self.text(&format!("[{}]", if alt.is_empty() { src } else { alt }));
            return;
        }
        let width = match size {
            Some((Some(w), _)) if w.trim().ends_with('%') => {
                let ratio = percent(w).unwrap_or(1.0);
                let file = self.file(&ext, Bytes::new(data));
                self.out.push_str(&format!("#box(image({}, alt: {}, width: {}%));", lit(&file), lit(alt), ratio * 100.0));
                return;
            }
            Some((Some(w), _)) => w.trim().trim_end_matches("px").parse::<f64>().unwrap_or(width),
            Some((None, Some(zoom))) => width * zoom,
            _ => width,
        };
        let file = self.file(&ext, Bytes::new(data));
        self.out.push_str(&format!("#box(pic({}, {width}, {}));", lit(&file), lit(alt)));
    }
}

/// `50%` as 0.5.
fn percent(s: &str) -> Option<f64> {
    s.trim().strip_suffix('%')?.trim().parse::<f64>().ok().map(|p| p / 100.0)
}

/// A colour as an inline style writes it, when it is one Typst can take.
fn css_color(s: &str) -> Option<Rgb> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix('#')
        && h.len() == 3
    {
        let long: String = h.chars().flat_map(|c| [c, c]).collect();
        return Rgb::parse(&format!("#{long}"));
    }
    if let Some(rgb) = s.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
        let v: Vec<u8> = rgb.split(',').filter_map(|p| p.trim().parse().ok()).collect();
        return (v.len() == 3).then(|| Rgb(v[0], v[1], v[2]));
    }
    Rgb::parse(s).or_else(|| {
        let named = [
            ("black", Rgb(0, 0, 0)),
            ("white", Rgb(255, 255, 255)),
            ("red", Rgb(255, 0, 0)),
            ("green", Rgb(0, 128, 0)),
            ("blue", Rgb(0, 0, 255)),
            ("yellow", Rgb(255, 255, 0)),
            ("orange", Rgb(255, 165, 0)),
            ("purple", Rgb(128, 0, 128)),
            ("gray", Rgb(128, 128, 128)),
            ("grey", Rgb(128, 128, 128)),
        ];
        named.iter().find(|(n, _)| n.eq_ignore_ascii_case(s)).map(|(_, c)| *c)
    })
}

/// An SVG's width in pixels, from its `viewBox` or `width`.
fn svg_width(svg: &str) -> Option<f64> {
    let open = &svg[svg.find("<svg")?..];
    let open = &open[..open.find('>')?];
    let attr = |name: &str| {
        let at = open.find(&format!(" {name}=\""))? + name.len() + 3;
        Some(&open[at..at + open[at..].find('"')?])
    };
    if let Some(w) = attr("width").and_then(|w| w.trim_end_matches("px").parse::<f64>().ok()) {
        return Some(w);
    }
    attr("viewBox")?.split([' ', ',']).filter(|p| !p.is_empty()).nth(2)?.parse().ok()
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

fn id(path: &str) -> FileId {
    FileId::new(RootedPath::new(VirtualRoot::Project, VirtualPath::new(path).expect("valid path")))
}

static LIBRARY: LazyLock<LazyHash<Library>> = LazyLock::new(|| LazyHash::new(Library::default()));

/// Typst's own fonts, for math, then the system's.
static FONTS: LazyLock<FontStore> = LazyLock::new(|| {
    let mut store = FontStore::new();
    store.extend(typst_assets::fonts().flat_map(|data| Font::iter(Bytes::new(data))).map(|f| {
        let info = f.info().clone();
        (f, info)
    }));
    store.extend(typst_kit::fonts::system());
    store
});

/// One export's compile: the markup, mitex's specs, and the files it reads.
struct Job {
    main: Source,
    files: HashMap<String, Bytes>,
}

impl World for Job {
    fn library(&self) -> &LazyHash<Library> {
        &LIBRARY
    }

    fn book(&self) -> &LazyHash<FontBook> {
        FONTS.book()
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            return Ok(self.main.clone());
        }
        let path = id.vpath().get_with_slash();
        match math::SPECS.iter().find(|(p, _)| *id.root() == VirtualRoot::Project && *p == path) {
            Some((_, text)) => Ok(Source::new(id, text.to_string())),
            None => Err(FileError::NotFound(path.into())),
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if *id.root() == VirtualRoot::Project
            && let Some(data) = self.files.get(&id.vpath().get_with_slash().to_string())
        {
            return Ok(data.clone());
        }
        self.source(id).map(Bytes::from_string)
    }

    fn font(&self, index: usize) -> Option<Font> {
        FONTS.font(index)
    }

    fn today(&self, _: Option<Duration>) -> Option<Datetime> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eqnum::Numbering;
    use crate::md::parse;
    use crate::palette::Scheme;

    fn look() -> Look {
        Look { scheme: Scheme::paper(), numbering: Numbering::Ams, math_fences: true, diagrams: true, ..Look::test() }
    }

    fn pdf_of(src: &str, settings: &PdfSettings) -> Result<Vec<u8>, String> {
        let doc = parse(src);
        let refs = inline::refs_of(&doc);
        let look = look();
        let paper = Paper { title: "t", look: &look, mono_font: "DejaVu Sans Mono", file: None, image_root: None, sheet: Sheet::Pages(settings) };
        pdf(&doc, &refs, &Options::all(), &paper)
    }

    #[test]
    fn indents_first_lines_and_keeps_line_breaks() {
        let doc = parse("one\ntwo\n\n- item\n");
        let refs = inline::refs_of(&doc);
        let look = Look { line_breaks: true, indent_first_line: true, ..look() };
        let set = PdfSettings::default();
        let paper = Paper { title: "t", look: &look, mono_font: "DejaVu Sans Mono", file: None, image_root: None, sheet: Sheet::Pages(&set) };
        let src = markup(&doc, &refs, &Options::all(), &paper).0;
        assert_eq!(src.matches("#h(2em)").count(), 1, "{src}");
        assert!(src.contains("#linebreak();"), "{src}");
        assert!(src.contains("#pad(left: 2em)[#item("), "{src}");
    }

    #[test]
    fn escapes_strings() {
        assert_eq!(lit("a\"b\\c\nd"), r#""a\"b\\c\nd""#);
    }

    #[test]
    fn numbers_pages_in_headers() {
        assert_eq!(template(" ", 16.0), "none");
        assert_eq!(
            template("T, \u{E001}/\u{E002}", 20.0),
            "context block(width: 100%, height: 20mm, align(center + horizon, text(size: 9pt, fill: fg)[#\"T, \";#counter(page).display();\
             #\"/\";#str(counter(page).final().first());#\"\";]))"
        );
    }

    #[test]
    fn reads_svg_widths() {
        assert_eq!(svg_width("<svg width=\"120px\" viewBox=\"0 0 10 10\">"), Some(120.0));
        assert_eq!(svg_width("<svg width=\"100%\" viewBox=\"-8 -8 240.5 100\">"), Some(240.5));
    }

    #[test]
    fn typesets_every_kind_of_block() {
        let src = "# Title\n\n[TOC]\n\nSome *em* **strong** `code` ~~del~~ ==mark== <u>u</u> <kbd>K</kbd> [link](https://x.y) [in](#part) \
                   $x^2$ and a note[^n].\n\n## Part\n\n- one\n  - two\n- [x] done\n\n3. three\n\n4. four\n\n> quote\n\n> [!TIP]\n> tip\n\n\
                   ```rust\nfn main() {\n\tlet s = \"\\\"\";\n}\n```\n\n$$\n\\begin{align} a &= b \\label{e} \\\\ c &= d \\end{align}\n$$\n\n\
                   See $\\eqref{e}$.\n\n| a | b |\n|:-|-:|\n| 1 | |\n\n```mermaid\ngraph LR\n  A --> B\n```\n\n---\n\n<div>html <b>block</b></div>\n\n\
                   $\\badcommand$\n\n[^n]: The note.\n";
        let set = PdfSettings {
            header: "t".into(),
            footer: "\u{E001} / \u{E002}".into(),
            subject: "s".into(),
            keywords: vec!["a".into(), "b".into()],
            break_on_h1: true,
            ..PdfSettings::default()
        };
        let pdf = pdf_of(src, &set).unwrap_or_else(|e| {
            let doc = parse(src);
            let refs = inline::refs_of(&doc);
            let look = look();
            let paper = Paper { title: "t", look: &look, mono_font: "DejaVu Sans Mono", file: None, image_root: None, sheet: Sheet::Pages(&set) };
            panic!("{e}\n{}", markup(&doc, &refs, &Options::all(), &paper).0)
        });
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn draws_an_image() {
        let doc = parse("# Hi\n\ntext\n");
        let refs = inline::refs_of(&doc);
        let look = look();
        let paper = Paper { title: "t", look: &look, mono_font: "DejaVu Sans Mono", file: None, image_root: None, sheet: Sheet::Image { width: 640.0, font: 24.0 } };
        let (w, h, rgba) = image(&doc, &refs, &Options::all(), &paper, 1.0, false).unwrap();
        assert_eq!(w, 640);
        assert!(h > 100 && h < 400, "{h}");
        assert_eq!(rgba.len(), (w * h * 4) as usize);
        // Auto quality keeps the screen's scale for a small image.
        assert_eq!(image(&doc, &refs, &Options::all(), &paper, 2.0, true).unwrap().0, 1280);
    }
}

/// Writes `$TYPO_SAMPLE` as a PDF and raw pixels beside it, for a look at them:
/// `TYPO_SAMPLE=doc.md cargo test sample_paper -- --ignored`.
#[cfg(test)]
#[test]
#[ignore]
fn sample_paper() {
    use crate::palette::Scheme;
    let path = std::path::PathBuf::from(std::env::var("TYPO_SAMPLE").unwrap());
    let doc = crate::md::parse(&std::fs::read_to_string(&path).unwrap());
    let refs = inline::refs_of(&doc);
    let look = Look { scheme: Scheme::paper(), numbering: eqnum::Numbering::Ams, math_fences: true, diagrams: true, ..Look::test() };
    let set = PdfSettings { footer: format!("{PAGE_NO} / {PAGE_COUNT}"), ..PdfSettings::default() };
    let mut paper =
        Paper { title: "sample", look: &look, mono_font: "JetBrainsMono Nerd Font", file: Some(&path), image_root: None, sheet: Sheet::Pages(&set) };
    std::fs::write(path.with_extension("typ"), markup(&doc, &refs, &Options::all(), &paper).0).unwrap();
    std::fs::write(path.with_extension("pdf"), pdf(&doc, &refs, &Options::all(), &paper).unwrap()).unwrap();
    paper.sheet = Sheet::Image { width: 640.0, font: 24.0 };
    let (w, h, rgba) = image(&doc, &refs, &Options::all(), &paper, 1.0, false).unwrap();
    // `magick -size WxH -depth 8 rgba:sample.rgba sample.png` makes a PNG of it.
    std::fs::write(path.with_extension(format!("{w}x{h}.rgba")), rgba).unwrap();
}
