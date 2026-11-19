//! The preview: a note drawn as a page from its Markdown tree (`md::tree`), styled as the
//! design's `renderPreview` draws it.

use std::collections::HashSet;

use rune::{Color, El, Family, Link, Span, TextStyle, Tx, col, el, icon, rich, row, text};

use super::V;
use super::widgets::{CHECK, label};
use crate::app::App;
use crate::md::{self, Align, Block, Inline};
use crate::theme::{self, Pal};

struct Look {
    serif: bool,
    size: f32,
    lh: f32,
    family: Family,
    mono: Family,
    /// The mono font's advance, in em.
    mono_w: f32,
    asc: f32,
    desc: f32,
}

impl Look {
    fn body(&self) -> TextStyle {
        self.weight(400.0)
    }
    fn weight(&self, w: f32) -> TextStyle {
        TextStyle::new(self.size, w).family(self.family).lh(self.size * self.lh)
    }
    /// Baseline below the top of a line box of this body text.
    fn baseline(&self, size: f32, lh: f32, asc: f32, desc: f32) -> f32 {
        (lh - (asc + desc) * size) / 2.0 + asc * size
    }
}

/// Rich text for one block: its string, spans, struck ranges and links.
#[derive(Default)]
struct Rich {
    text: String,
    spans: Vec<Span>,
    strikes: Vec<(usize, usize, Option<Color>)>,
    links: Vec<(std::ops::Range<usize>, String)>,
    /// The spaces that pad inline code and tags: drawn, not copied.
    pads: Vec<std::ops::Range<usize>>,
}

/// The style that inline markup around a run gives it.
#[derive(Clone, Copy, Default)]
struct Ctx {
    bold: bool,
    italic: bool,
    color: Option<Color>,
    underline: Option<Color>,
}

impl Rich {
    fn push(&mut self, s: &str, cx: Ctx) {
        let a = self.text.len();
        self.text.push_str(s);
        let b = self.text.len();
        if a < b && (cx.bold || cx.italic || cx.color.is_some() || cx.underline.is_some()) {
            self.spans.push(Span { weight: cx.bold.then_some(600.0), italic: cx.italic, color: cx.color, underline: cx.underline, ..Span::new(a, b) });
        }
    }

    /// The design's `inl()`: inline Markdown to styled runs.
    fn inlines(&mut self, v: &[Inline], cx: Ctx, p: &Pal, look: &Look, names: &md::Names) {
        for i in v {
            match i {
                Inline::Text(t) => self.push(t, cx),
                Inline::Break => self.push("\n", cx),
                Inline::Code(t) => {
                    // `padding: .12em .38em` at .8em: the background grows by the vertical
                    // padding, and spaces sized to the rest of the horizontal padding take room.
                    let size = look.size * 0.8;
                    self.chip(t, size, look.mono_w, 0.12 * size, 0.38 * size, cx.color, p.code);
                }
                Inline::Strong(k) => self.inlines(k, Ctx { bold: true, ..cx }, p, look, names),
                Inline::Emph(k) => self.inlines(k, Ctx { italic: true, ..cx }, p, look, names),
                Inline::Strike(k) => {
                    let a = self.text.len();
                    self.inlines(k, Ctx { color: Some(p.mute), ..cx }, p, look, names);
                    let b = self.text.len();
                    self.strikes.push((a, b, Some(p.mute)));
                }
                Inline::Link { url, kids } => {
                    let a = self.text.len();
                    let lc = Ctx { color: Some(p.acc), underline: Some(p.acc), ..cx };
                    if kids.is_empty() {
                        self.push(url, lc);
                    } else {
                        self.inlines(kids, lc, p, look, names);
                    }
                    let b = self.text.len();
                    self.links.push((a..b, format!("url:{url}")));
                }
                Inline::Wiki { target, shown } => {
                    let ok = names.resolve(target).is_some();
                    let a = self.text.len();
                    self.push(shown, Ctx { color: Some(if ok { p.acc } else { p.mute }), underline: Some(if ok { p.link_line } else { p.mark }), ..cx });
                    let b = self.text.len();
                    self.links.push((a..b, format!("wiki:{target}")));
                }
                // An image inside a sentence shows its alt text. Lines of images are blocks.
                Inline::Image(img) => {
                    self.push(if img.alt.is_empty() { "image" } else { &img.alt }, Ctx { italic: true, color: Some(p.mute), ..Ctx::default() });
                }
                Inline::Tag(t) => {
                    let size = look.size * 0.72;
                    let a = self.text.len();
                    self.chip(&format!("#{t}"), size, look.mono_w, 0.18 * size, 0.45 * size, Some(p.acc), p.soft);
                    let b = self.text.len();
                    self.links.push((a..b, format!("tag:{t}")));
                }
            }
        }
    }

    /// A run with a padded background, as CSS draws inline code and tags.
    fn chip(&mut self, t: &str, size: f32, mono_w: f32, pad_y: f32, pad_x: f32, color: Option<Color>, bg: Color) {
        // A mono space is `mono_w` em wide, so a space at this size is exactly the padding left
        // over after the background's own outset.
        let side = (pad_x - pad_y).max(0.0);
        let space = side / mono_w;
        let a = self.text.len();
        self.text.push(' ');
        let b = self.text.len();
        self.text.push_str(t);
        let c = self.text.len();
        self.text.push(' ');
        let d = self.text.len();
        self.pads.push(a..b);
        self.pads.push(c..d);
        if space > 0.5 {
            self.spans.push(Span { mono: true, size: Some(space), bg: Some(bg), bg_pad: pad_y, ..Span::new(a, b) });
            self.spans.push(Span { mono: true, size: Some(space), bg: Some(bg), bg_pad: pad_y, ..Span::new(c, d) });
        }
        self.spans.push(Span { mono: true, size: Some(size), color, bg: Some(bg), bg_pad: pad_y, ..Span::new(b, c) });
    }

    /// The block's text, which people can select and copy (`selectable`).
    fn el(self, style: TextStyle, color: Color, p: &Pal) -> El {
        let mut e = rich(if self.text.is_empty() { " ".to_string() } else { self.text }, self.spans).font(style).color(color).wrap().span_radius(4.0);
        for (a, b, c) in self.strikes {
            e = e.strike(a, b, c);
        }
        for (r, id) in self.links {
            e = e.link(Link::new(r, id));
        }
        for r in self.pads {
            e = e.uncopied(r);
        }
        e.on_link(on_link).selectable(true).selection_color(p.sel_editor)
    }
}

fn on_link(s: &mut App, id: &str, tx: &mut Tx) {
    if let Some(t) = id.strip_prefix("wiki:") {
        s.open_link(t, tx);
    } else if let Some(t) = id.strip_prefix("tag:") {
        s.search_tag(t);
    } else if let Some(u) = id.strip_prefix("url:") {
        if let Err(e) = tx.open_url(u) {
            s.notice = Some(format!("Could not open {u}: {e}"));
        }
    }
}

/// What draws one note.
struct Pv<'a> {
    v: V<'a>,
    /// The note drawn, and what its keys start with: "" in the focused pane.
    note: &'a str,
    prefix: &'a str,
    look: Look,
    names: &'a md::Names,
    used: HashSet<usize>,
    n: usize,
    /// The list marker ("- ", "1. ") that a copy puts before the next block's text.
    lead: Option<String>,
}

impl Pv<'_> {
    /// `pv-{line}` for the outermost block on a line, which the outline and search scroll to;
    /// a key of its own for a block nested on the same line.
    fn key(&mut self, line: usize) -> String {
        if self.used.insert(line) {
            format!("{}pv-{line}", self.prefix)
        } else {
            self.n += 1;
            format!("{}b-{}", self.prefix, self.n)
        }
    }
}

/// Where a block sits.
#[derive(Clone, Copy, Default)]
struct Within {
    quote: bool,
    /// In an item of a tight list: paragraphs take no margins.
    tight: bool,
    /// In a done task.
    done: bool,
    /// How many lists hold it.
    depth: usize,
}

/// Copying puts a blank line between blocks, as a browser copies paragraphs; `lead` is a list
/// item's marker.
fn copyable(e: El, breaks: u8, lead: Option<String>) -> El {
    let e = e.copy_breaks(breaks, breaks);
    match lead {
        Some(l) => e.copy_prefix(l),
        None => e,
    }
}

pub fn preview(v: V) -> El {
    let note = v.s.active.as_deref().unwrap_or("");
    preview_of(v, v.s.active_text(), note, "")
}

/// A note's preview. Its keys start with `prefix`, so a second pane's don't clash.
pub fn preview_of<'a>(v: V<'a>, text: &str, note: &'a str, prefix: &'a str) -> El {
    let s = v.s;
    let set = s.settings();
    let serif = set.preview_font == "serif";
    let size = set.size as f32 + if serif { 3.0 } else { 1.0 };
    let look = Look {
        serif,
        size,
        lh: if serif { 1.62 } else { 1.65 },
        family: if serif { theme::SERIF } else { theme::SANS },
        mono: theme::editor_font(&set.font).family,
        mono_w: 0.6,
        asc: if serif { 0.735 } else { 0.97 },
        desc: if serif { 0.265 } else { 0.25 },
    };
    let blocks = md::tree(text);
    let mut pv = Pv { v, note, prefix, look, names: &s.names, used: HashSet::new(), n: 0, lead: None };
    // Collapsed top margin left after the breadcrumb's 30 px bottom margin.
    let mut prev_mb: f32 = if v.focus() { 0.0 } else { 30.0 };
    let mut first = true;
    // One selection across the note's blocks, as on a web page.
    let mut out = col().key(format!("{prefix}preview")).w_full().selection_group(true);
    for b in blocks.iter() {
        let (mt, mb, el_) = block(&mut pv, b, Within::default());
        let gap = if first { (mt - prev_mb).max(0.0) } else { mt.max(prev_mb) };
        first = false;
        out = out.child(el_.mt(gap));
        prev_mb = mb;
    }
    out
}

/// Blocks in a column, with their margins collapsed as CSS collapses them. The first block's
/// top margin and the last one's bottom margin fall inside the box that holds them.
fn stack(pv: &mut Pv, blocks: &[Block], w: Within) -> El {
    let mut c = col().w_full();
    let mut prev: Option<f32> = None;
    for b in blocks {
        let (mt, mb, e) = block(pv, b, w);
        c = c.child(e.mt(prev.map_or(0.0, |p| mt.max(p))));
        prev = Some(mb);
    }
    c
}

/// One block with its CSS top and bottom margins (collapsed by the caller).
fn block(pv: &mut Pv, b: &Block, w: Within) -> (f32, f32, El) {
    let v = pv.v;
    let p = v.p;
    let em = pv.look.size;
    let body = pv.look.body();
    let lead = pv.lead.take();
    match b {
        Block::Props { line, props } => {
            let key = pv.key(*line);
            (0.0, em * 1.6, properties(pv, key, props))
        }
        Block::Image { line, images } => {
            let mut c = col().key(pv.key(*line)).w_full().gap(em * 0.6);
            for (i, img) in images.iter().enumerate() {
                c = c.child(picture(v, img, &pv.look, pv.note).key(format!("img-{i}")));
            }
            (em * 0.4, em * 0.95, c)
        }
        Block::Pre { line, text: t } => {
            let style = TextStyle::new((em * 0.74).round(), 400.0).family(pv.look.mono).lh((em * 0.74).round() * 1.65);
            let e = el()
                .key(pv.key(*line))
                .w_full()
                .radius(8.0)
                .fill(p.code)
                .scroll_x()
                .child(el().py(14.0).px(18.0).child(copyable(
                    text(if t.is_empty() { " ".to_string() } else { t.clone() }).font(style).color(p.ink2).selectable(true).selection_color(p.sel_editor),
                    2,
                    lead,
                )));
            (em * 1.2, em * 1.3, e)
        }
        Block::Heading { line, level, inlines } => {
            // Levels 4 to 6 look like 3.
            let (k, lh, wt, mt, mb, track) = match level {
                1 => (1.72, 1.15, if pv.look.serif { 500.0 } else { 600.0 }, 0.0, 0.7, -0.012),
                2 => (1.3, 1.25, 600.0, 1.7, 0.55, 0.0),
                _ => (1.08, 1.3, 600.0, 1.5, 0.45, 0.0),
            };
            let size = em * k;
            let style = TextStyle::new(size, wt).family(pv.look.family).lh(size * lh).tracking(size * track);
            let mut r = Rich::default();
            r.inlines(inlines, Ctx::default(), p, &pv.look, pv.names);
            let e = copyable(r.el(style, p.ink, p), 2, lead).key(pv.key(*line)).w_full().role("heading").level(*level as u32).label(md::plain(inlines).trim().to_string());
            (size * mt, size * mb, e)
        }
        Block::Rule { line } => (em * 2.2, em * 2.2, el().key(pv.key(*line)).h(1.0).w_full().fill(p.line)),
        Block::Quote { line, blocks } => {
            let key = pv.key(*line);
            let inner = stack(pv, blocks, Within { quote: true, ..w });
            let e = el().key(key).w_full().pt(em * 0.1).pb(em * 0.1).pl(em * 1.1 + 2.0).border_sides([0.0, 0.0, 0.0, 2.0], p.mark).child(inner);
            (em * 1.3, em * 1.3, e)
        }
        Block::List { line, ordered, start, delim, tight, items } => {
            let mut c = col().key(pv.key(*line)).w_full().role("list");
            let gap = if *tight { em * 0.25 } else { em * 0.95 };
            let text_base = pv.look.baseline(em, em * pv.look.lh, pv.look.asc, pv.look.desc);
            for (i, it) in items.iter().enumerate() {
                let done = it.task == Some(true);
                let mark = match it.task {
                    Some(true) => "- [x] ".to_string(),
                    Some(false) => "- [ ] ".to_string(),
                    None if *ordered => format!("{}{delim} ", start + i),
                    None => "- ".to_string(),
                };
                let marker = if let Some(done) = it.task {
                    let sz = em * 0.85;
                    let top = text_base - sz + em * 0.1;
                    let line_no = it.line;
                    let mut bx = row()
                        .key("box")
                        .size(sz, sz)
                        .mt(top)
                        .center()
                        .radius(3.0)
                        .border(1.5, if done { p.acc } else { p.mark })
                        .role("checkbox")
                        .checked(done)
                        .cursor(rune::Cursor::Pointer)
                        .on_press(move |s: &mut App| s.toggle_task(line_no));
                    if done {
                        bx = bx.fill(p.acc).child(icon(&CHECK, 9.0).color(p.on_acc));
                    }
                    bx
                } else if *ordered {
                    let ns = em * 0.78;
                    let nlh = ns * pv.look.lh;
                    let shift = text_base - pv.look.baseline(ns, nlh, 1.025, 0.275);
                    let n = format!("{}{delim}", start + i);
                    el().key("num").min_w(ns * 1.3).mt(shift).child(text(n).font(TextStyle::new(ns, 400.0).family(pv.look.mono).lh(nlh)).color(p.mute))
                } else {
                    row().key("bullet").w(em).justify_center().child(text("•").font(body).color(p.acc))
                };
                pv.lead = Some(format!("{}{mark}", "  ".repeat(w.depth)));
                let inner = stack(pv, &it.blocks, Within { tight: *tight, done, depth: w.depth + 1, ..w });
                pv.lead = None;
                let li = row()
                    .key(format!("li-{}", it.line))
                    .w_full()
                    .gap(em * 0.65)
                    .mt(if i == 0 { 0.0 } else { gap })
                    .child(marker)
                    .child(inner.grow(1.0).shrink().min_w(0.0));
                c = c.child(li);
            }
            (em * 0.25, em, c)
        }
        Block::Table { line, align, rows } => {
            let key = pv.key(*line);
            (0.0, em * 0.95, table(pv, key, align, rows))
        }
        Block::Para { line, inlines } => {
            let (mt, mb) = if w.tight { (0.0, 0.0) } else { (0.0, em * 0.95) };
            if let Some(tags) = tag_words(inlines) {
                let key = pv.key(*line);
                return (mt, mb, tag_line(v, &pv.look, key, tags));
            }
            let mut r = Rich::default();
            let cx = Ctx { italic: w.quote, color: w.done.then_some(p.mute), ..Ctx::default() };
            r.inlines(inlines, cx, p, &pv.look, pv.names);
            if w.done {
                r.strikes.push((0, r.text.len(), Some(p.mark)));
            }
            let (style, color) = if w.quote { (body.italic(), p.ink2) } else { (body, p.ink) };
            // A tight list's items copy on lines of their own.
            (mt, mb, copyable(r.el(style, color, p), if w.tight { 1 } else { 2 }, lead).key(pv.key(*line)).w_full())
        }
    }
}

/// A table as a browser lays one out by default: each column as wide as its widest cell on one
/// line when the page has room, and otherwise narrower, in proportion to how much its text can
/// wrap, but never narrower than its longest word.
fn table(pv: &mut Pv, key: String, align: &[Align], rows: &[Vec<Vec<Inline>>]) -> El {
    let p = pv.v.p;
    let em = pv.look.size;
    let (pad_x, pad_y) = (em * 0.75, em * 0.4);
    let mut cells: Vec<Vec<(Rich, TextStyle)>> = Vec::new();
    let mut min = vec![0.0f32; align.len()];
    let mut max = vec![0.0f32; align.len()];
    for (ri, r) in rows.iter().enumerate() {
        let style = pv.look.weight(if ri == 0 { 600.0 } else { 400.0 });
        let mut out = Vec::new();
        for (ci, c) in r.iter().enumerate() {
            let mut rich = Rich::default();
            rich.inlines(c, Ctx::default(), p, &pv.look, pv.names);
            let (a, b) = rune::text::content_widths(&rich.text, &rich.spans, style);
            // A cell's border and padding, and a point for rounding.
            let edge = 2.0 * pad_x + 1.0;
            min[ci] = min[ci].max(a.ceil() + edge);
            max[ci] = max[ci].max(b.ceil() + edge);
            out.push((rich, style));
        }
        cells.push(out);
    }
    let mut t = col().key(key).w_full().role("table");
    for (ri, r) in cells.into_iter().enumerate() {
        let mut tr = row().key(format!("r-{ri}")).w_full().items_stretch().role("row");
        for (ci, (rich, style)) in r.into_iter().enumerate() {
            let a = match align.get(ci) {
                Some(Align::Center) => rune::Align::Center,
                Some(Align::Right) => rune::Align::End,
                _ => rune::Align::Start,
            };
            let mut cell = el()
                .key(format!("c-{ci}"))
                .grow(max[ci] - min[ci])
                .min_w(min[ci])
                .max_w(max[ci])
                .px(pad_x)
                .py(pad_y)
                .border_sides([if ri == 0 { 1.0 } else { 0.0 }, 1.0, 1.0, if ci == 0 { 1.0 } else { 0.0 }], p.line)
                .role("cell")
                .child(rich.el(style, p.ink, p).w_full().align(a));
            if ri == 0 {
                cell = cell.fill(p.code);
            }
            tr = tr.child(cell);
        }
        t = t.child(tr);
    }
    t
}

/// Front matter as a small table of properties, as Obsidian shows it above a note.
fn properties(pv: &mut Pv, key: String, props: &[(String, Vec<Inline>)]) -> El {
    let p = pv.v.p;
    let em = pv.look.size;
    let size = (em * 0.8).round();
    let style = theme::sans(size, 400.0);
    let widest = props.iter().map(|(k, _)| rune::text::advance(k, style)).fold(0.0, f32::max);
    let kw = widest.ceil().min(em * 10.0);
    let mut c = col().key(key).w_full().gap(size * 0.35).pb(em * 0.9).border_sides([0.0, 0.0, 1.0, 0.0], p.line).role("table").label("Properties");
    for (i, (k, vals)) in props.iter().enumerate() {
        let mut r = Rich::default();
        r.inlines(vals, Ctx::default(), p, &pv.look, pv.names);
        c = c.child(
            row()
                .key(format!("p-{i}"))
                .w_full()
                .gap(em * 0.9)
                .items_start()
                .role("row")
                .child(label(k.clone(), style, p.mute).wrap().w(kw).selectable(true).selection_color(p.sel_editor))
                .child(r.el(style, p.ink2, p).grow(1.0).shrink().min_w(0.0)),
        );
    }
    c
}

/// One image at its own size, or narrower to fit the column. A box holds its place while it
/// loads, and a line says so when there is no file.
fn picture(v: V, img: &md::Img, look: &Look, note_path: &str) -> El {
    let p = v.p;
    let s = v.s;
    let note = |msg: String| {
        row()
            .w_full()
            .px(14.0)
            .py(10.0)
            .radius(6.0)
            .fill(p.code)
            .child(label(msg, theme::sans(look.size * 0.8, 400.0), p.mute).wrap().grow(1.0).shrink())
    };
    let handle = match s.image_source_for(img, Some(note_path)) {
        crate::images::Source::File(path) => s.images.file(&path).map(Ok),
        crate::images::Source::Url(url) => match s.images.remote(&url) {
            crate::images::Remote::Ready(i) => Some(Ok(i)),
            crate::images::Remote::Loading => None,
            crate::images::Remote::Failed(e) => Some(Err(format!("Could not load {url}: {e}"))),
        },
        crate::images::Source::Missing(src) => Some(Err(format!("No image at {src}"))),
    };
    let alt = if img.alt.is_empty() { "Image".to_string() } else { img.alt.clone() };
    match handle {
        Some(Ok(i)) if i.error().is_some() => note(format!("Could not show {}: {}", img.src, i.error().map_or("", |e| e.0.as_str()))),
        Some(Ok(i)) => {
            // A pending image paints when its pixels arrive. Its own width caps the picture.
            let mut pic = el().pixels(i.pixels()).w_full().radius(6.0).clip_rounded(6.0).role("img").label(alt.clone()).help(alt);
            if let Some([w, _]) = i.size() {
                pic = pic.max_w(w as f32);
            }
            row().w_full().child(pic)
        }
        Some(Err(msg)) => note(msg),
        _ => el().w_full().h(look.size * 6.0).radius(6.0).fill(p.code).role("img").label(alt),
    }
}

/// The tags of a paragraph of nothing but `#tags`.
fn tag_words(v: &[Inline]) -> Option<Vec<&str>> {
    let mut tags = Vec::new();
    for i in v {
        match i {
            Inline::Tag(t) => tags.push(t.as_str()),
            Inline::Break => {}
            Inline::Text(t) if t.trim().is_empty() => {}
            _ => return None,
        }
    }
    (!tags.is_empty()).then_some(tags)
}

/// Tags drawn as chips with CSS's exact padding, on the paragraph's baseline.
fn tag_line(v: V, look: &Look, key: String, tags: Vec<&str>) -> El {
    let p = v.p;
    let em = look.size;
    let lh = em * look.lh;
    let (a, d) = (theme::px(look.asc * em), theme::px(look.desc * em));
    let base = (lh - a - d) / 2.0 + a;
    let size = em * 0.72;
    let (ma, md_) = theme::metrics(look.mono);
    let (ca, cd) = (theme::px(ma * size), theme::px(md_ * size));
    let pad_y = 0.18 * size;
    let space = em * 0.25;
    let mut r = row().key(key).w_full().min_h(lh).flex_wrap().gap(space).items_start();
    for (i, t) in tags.into_iter().enumerate() {
        let w = format!("#{t}");
        let tag = t.to_string();
        r = r.child(
            row()
                .key(format!("tag-{i}"))
                .mt(base - ca - pad_y)
                .py(pad_y)
                .px(0.45 * size)
                .radius(4.0)
                .fill(p.soft)
                .cursor(rune::Cursor::Pointer)
                .role("link")
                .label(w.clone())
                .on_press(move |s: &mut App| s.search_tag(&tag))
                .child(text(w).font(TextStyle::new(size, 400.0).family(look.mono).lh(ca + cd)).color(p.acc)),
        );
    }
    r
}

#[cfg(test)]
mod tests {
    use rune::{Env, Key, KeyEvent, Mods, Runtime};

    use crate::app::{App, Mode};

    const NOTE: &str = "# Title\n\nFirst `code` para.\n\n- one\n- [x] two\n\nLast para #demo.";

    fn frames(rt: &mut Runtime<App>) {
        rt.frame(1.0 / 60.0);
        rt.settle_all();
        for _ in 0..2 {
            rt.frame(1.0 / 60.0);
        }
    }

    fn command(rt: &mut Runtime<App>, c: char) {
        rt.key(KeyEvent { key: Key::Command(c), mods: Mods { cmd: true, ..Mods::default() } });
        frames(rt);
    }

    /// A drag selects across the preview's blocks, and Cmd+A then Cmd+C copies the whole note
    /// as text: a blank line between blocks, list markers, no padding around inline code or tags.
    /// Links in the text still follow a click.
    #[test]
    fn preview_text_selects_and_copies() {
        let _env = crate::scroll_tests::ENV.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("typo-preview-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // SAFETY: the ENV lock keeps other tests that read these variables out.
        unsafe {
            std::env::set_var("HOME", &dir);
            std::env::set_var("XDG_CONFIG_HOME", dir.join("cfg"));
            std::env::set_var("XDG_DATA_HOME", dir.join("data"));
        }
        let mut rt = Runtime::new(crate::app::init(), crate::ui::view, (1280.0, 800.0), Env { system_dark: false, scale: 1.0 });
        let clipboard = rune::clipboard::memory();
        rt.use_clipboard(&clipboard);
        rt.frame(1.0 / 60.0);
        rt.update(|s, _| {
            let a = s.active.clone().unwrap();
            s.set_text(&a, NOTE.to_string());
            s.mode = Mode::Preview;
        });
        frames(&mut rt);

        // Drag from the title's first letter into the last paragraph.
        let (title, _) = rt.presented("pv-0").expect("the title shows");
        let (last, _) = rt.presented("pv-7").expect("the last paragraph shows");
        let (x0, y0) = (title[0] + 2.0, title[1] + title[3] / 2.0);
        let (x1, y1) = (last[0] + 30.0, last[1] + last[3] / 2.0);
        rt.pointer_move(x0, y0);
        rt.pointer_down(x0, y0);
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            rt.pointer_move(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
            rt.frame(1.0 / 60.0);
        }
        rt.pointer_up(x1, y1);
        frames(&mut rt);
        let dragged = rt.selected_text().unwrap_or_default();
        assert!(dragged.starts_with("Title") && dragged.contains("First code para.") && dragged.ends_with("Las"), "the drag selects across blocks: {dragged:?}");

        command(&mut rt, 'a');
        command(&mut rt, 'c');
        let copied = clipboard.text().map(|t| t.to_string());
        assert_eq!(copied.as_deref(), Some("Title\n\nFirst code para.\n\n- one\n- [x] two\n\nLast para #demo."));

        // A click on a link in selectable text still follows it.
        rt.update(|_, tx| tx.clear_text_selection());
        frames(&mut rt);
        let (x, y) = rt.find("tag:demo").expect("the tag shows");
        rt.pointer_move(x, y);
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        frames(&mut rt);
        let mut query = String::new();
        rt.update(|s, _| query = s.query.text.clone());
        assert_eq!(query, "#demo", "a click on the tag searches for it");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
