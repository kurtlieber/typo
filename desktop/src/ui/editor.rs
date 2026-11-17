//! The editor body: sidebar, the sheet with the note, the outline and the status bar.
//!
//! Source mode works as the design does: highlighted lines draw underneath, and a text field
//! with transparent text lies over them and takes the typing. Both lay out the same text in the
//! same font at the same width, so the caret and selection land on the drawn glyphs.

use rune::{Color, El, Key, KeyEvent, ScrollFade, ScrollSpec, Span, TextEdit, TextStyle, Tx, col, el, icon, rich, row, text, text_input};

use super::widgets::{CLOSE, label};
use super::{V, outline, preview, sidebar};
use crate::app::{App, Mode};
use crate::keys::k;
use crate::md::{self, Tok};
use crate::tabs::{Dir, Split};
use crate::theme::{self, Pal, mono, sans};

pub fn body(v: V) -> El {
    let p = v.p;
    let focus = v.focus();
    let left = v.left_visible();
    // Under an active first tab the corner is square, so the tab's left border runs straight
    // down into the sheet's.
    let first = v.s.tabs.iter().find(|t| v.s.files().contains_key(*t));
    let round = left && (focus || first.is_none() || first != v.s.active.as_ref());
    let corner = if round { [10.0, 0.0, 0.0, 0.0] } else { [0.0; 4] };
    let mut sheet = col().key("sheet").grow(1.0).min_w(0.0).min_h(0.0).fill(p.bg).clip_corners(corner).corners(corner);
    if v.has_doc() {
        // Image files dropped on the note link at the caret. Wayland gives no file drops yet,
        // so there a copied file or image goes in with paste.
        sheet = sheet
            .on_drop_files(|s: &mut App, files, tx| {
                s.drop_over = false;
                s.insert_images(files, tx);
            })
            .accepts_extensions(&md::IMAGE_EXT)
            .on_drop_hover(|s: &mut App, over, _| s.drop_over = over);
        if v.s.drop_over {
            sheet = sheet.box_shadow(rune::Shadow { x: 0.0, y: 0.0, blur: 0.0, spread: 2.0, color: p.acc, inset: true });
        }
    }
    let top = if focus { 0.0 } else { 1.0 };
    let side = if left { 1.0 } else { 0.0 };
    if top + side > 0.0 {
        sheet = sheet.border_sides([top, 0.0, 0.0, side], p.line).border_layout();
    }
    let area = match v.s.split_view().filter(|_| !focus && v.has_doc()) {
        Some(sp) => panes(v, sp),
        None => doc_area(v, false, true),
    };
    let mut inner = row().key("sheet-row").grow(1.0).min_h(0.0).w_full().child(area);
    if v.right_visible() {
        inner = inner.child(outline::outline(v));
    }
    sheet = sheet.child(inner);
    if v.has_doc() && !focus {
        sheet = sheet.child(outline::status(v));
    }
    if focus {
        sheet = sheet.child(focus_hint(v));
    }
    let mut r = row().key("body").grow(1.0).min_h(0.0).w_full();
    if left {
        r = r.child(sidebar::sidebar(v));
    }
    r.child(sheet)
}

/// The focused note's area: the note, its toolbar and the find bar. In a split it has a close
/// button and a line in the accent across its top.
fn doc_area(v: V, split: bool, full: bool) -> El {
    let p = v.p;
    let mut area = col().key("doc-area").grow(1.0).min_w(0.0).min_h(0.0).child(doc_scroll(v));
    if full {
        area = area.h_full();
    }
    // In a split one toolbar spans both panes: a pane is narrower than the toolbar.
    if shows_toolbar(v) && !split {
        area = area.child(toolbar_dock(v));
    }
    if v.s.find.is_some() && v.has_doc() {
        area = area.child(super::find::bar(v, split));
    }
    if split {
        area = area
            .child(el().key("pane-mark").pin(Some(0.0), Some(0.0), Some(0.0), None).h(2.0).fill(p.acc.alpha(0.55)))
            .child(pane_close(v, "pane-x-a", true));
    }
    area
}

/// Two panes, side by side or one over the other, with a line between them.
fn panes(v: V, sp: &Split) -> El {
    let p = v.p;
    let right = sp.dir == Dir::Right;
    // Each pane pads its note for half the window.
    let half = if right { V { vw: v.vw / 2.0, ..v } } else { V { vh: v.vh / 2.0, ..v } };
    let a = doc_area(half, true, right);
    let b = other_pane(half, &sp.other, right);
    let line = if right { el().key("pane-line").w(1.0).h_full().fill(p.line) } else { el().key("pane-line").h(1.0).w_full().fill(p.line) };
    let (first, second) = if sp.other_first { (b, a) } else { (a, b) };
    let c = if right { row() } else { col() };
    let c = c.key("panes").grow(1.0).min_w(0.0).min_h(0.0).h_full().child(first).child(line).child(second);
    if shows_toolbar(v) { c.child(toolbar_dock(v)) } else { c }
}

/// The pane that isn't focused: its note, drawn without an editor. A press in it focuses it.
fn other_pane(v: V, note: &str, full: bool) -> El {
    let s = v.s;
    let text = s.note_text(note);
    let size = s.settings().size as f32;
    let pad_x = clamp(20.0, v.vw * 0.05, 56.0);
    let mut column = col().key("o-doc-col").w_full().pt(34.0).pb(v.vh * 0.38).px(pad_x);
    if s.settings().width == "readable" {
        column = column.max_w(size * 0.6 * 72.0 + 2.0 * pad_x);
    }
    column = column.child(static_crumb(v, note, "o-crumb"));
    column = column.child(if s.note_locked(note) {
        label("Locked", sans(14.0, 400.0), v.p.mute).key("o-locked")
    } else if s.mode == Mode::Source {
        el().key("o-source").w_full().child(mirror(v, text, "o-", None, &[], None))
    } else {
        preview::preview_of(v, text, note, "o-")
    });
    let content = col().key("o-doc-content").w_full().items_center().inert().child(column);
    let scroll = col()
        .key("doc-scroll-2")
        .grow(1.0)
        .min_w(0.0)
        .min_h(0.0)
        .w_full()
        .scroll(ScrollSpec { stick_bottom: false })
        .on_press_tx(|s: &mut App, tx| s.focus_other_pane(tx))
        .child(content);
    let mut area = col().key("doc-area-2").grow(1.0).min_w(0.0).min_h(0.0);
    if full {
        area = area.h_full();
    }
    area.child(scroll).child(pane_close(v, "pane-x-b", false))
}

fn pane_close(v: V, key: &str, focused: bool) -> El {
    let p = v.p;
    row()
        .key(key)
        .pin(None, Some(8.0), Some(10.0), None)
        .size(22.0, 22.0)
        .center()
        .radius(5.0)
        .hover(p.hover)
        .role("button")
        .label("Close pane")
        .help("Close pane")
        .on_press_tx(move |s: &mut App, tx| s.close_pane(focused, tx))
        .child(icon(&CLOSE, 8.0).color(p.mute))
}

/// The note's folder and name, as the crumb shows them, without the rename button.
fn static_crumb(v: V, note: &str, key: &str) -> El {
    let p = v.p;
    let style = mono(12.0, 400.0);
    let dir = md::dir(note).to_string();
    let mut r = row().key(key).h(24.0).mb(30.0).gap(6.0).center_y();
    if !dir.is_empty() {
        r = r.child(label(dir, style, p.mute)).child(label("/", style, p.mark));
    }
    r.child(row().child(label(md::base(note), style, p.ink2)).child(label(".md", style, p.mark)))
}

/// The editor font at the configured size, with the design's 1.75 line height.
pub fn ed_style(v: V) -> TextStyle {
    let set = v.s.settings();
    let size = set.size as f32;
    TextStyle::new(size, 400.0).family(theme::editor_font(&set.font).family).lh(size * 1.75).features("'liga' 0, 'clig' 0, 'calt' 0")
}

fn clamp(lo: f32, x: f32, hi: f32) -> f32 {
    x.max(lo).min(hi)
}

fn doc_scroll(v: V) -> El {
    let s = v.s;
    let focus = v.focus();
    let mut content = col().key("doc-content").w_full().items_center();
    if v.has_doc() {
        let size = s.settings().size as f32;
        let pad_x = if focus { clamp(20.0, v.vw * 0.06, 64.0) } else { clamp(20.0, v.vw * 0.05, 56.0) };
        let (pad_t, pad_b) = if focus { (v.vh * 0.16, v.vh * 0.48) } else { (34.0, v.vh * 0.38) };
        let ch = if focus { 68.0 } else { 72.0 };
        let mut column = col()
            .key("doc-col")
            .w_full()
            .pt(pad_t)
            .pb(pad_b)
            .px(pad_x)
            .on_press_tx(|s: &mut App, tx| {
                // A click below the text puts the caret at the end, as the design's column does.
                if s.mode == Mode::Source {
                    let e = s.cur_edit();
                    let n = e.text.len();
                    e.set_selection(n, n);
                    s.focus_editor(tx);
                }
            });
        if s.settings().width == "readable" {
            column = column.max_w(size * 0.6 * ch + 2.0 * pad_x);
        }
        if !focus {
            column = column.child(crumb(v));
        }
        let locked = s.active.as_deref().is_some_and(|a| s.note_locked(a));
        column = column.child(if locked {
            super::lock::panel(v)
        } else if s.mode == Mode::Source {
            source(v)
        } else {
            preview::preview(v)
        });
        content = content.child(column);
    } else if s.files().is_empty() {
        content = content.min_h(v.vh - 42.0 - 1.0).justify_center().padding(40.0).child(empty_vault(v));
    } else {
        content = content.min_h(v.vh - 42.0 - 1.0).justify_center().padding(40.0).child(no_tab(v));
    }
    // A scroll viewport is a column: in a row its content would stretch to the viewport's
    // height and leave nothing to scroll.
    let mut scroll = col().key("doc-scroll").grow(1.0).min_w(0.0).min_h(0.0).w_full().scroll(ScrollSpec { stick_bottom: false });
    if shows_toolbar(v) {
        // Text fades out at the bottom, so the toolbar floats on a clear band. The caret stays
        // above the toolbar while you type.
        let band = TOOLBAR_BOTTOM + TOOLBAR_H;
        scroll = scroll.scroll_fade(ScrollFade::strength(band + 12.0).end_only()).scroll_margin(band + 16.0);
    }
    scroll.child(content)
}

/// The space under the floating toolbar, above the status bar.
const TOOLBAR_BOTTOM: f32 = 16.0;
/// The height of the floating toolbar.
const TOOLBAR_H: f32 = 36.0;

/// Whether the formatting toolbar shows: source mode, the setting on, a note open and not
/// locked, and not in focus mode.
fn shows_toolbar(v: V) -> bool {
    let s = v.s;
    let locked = s.active.as_deref().is_some_and(|a| s.note_locked(a));
    v.has_doc() && !v.focus() && s.mode == Mode::Source && s.settings().toolbar && !locked
}

/// The toolbar's lock: a plain note gets a password, and an unlocked note locks again. The
/// accent marks a note that is locked and open now.
fn lock_button(v: V) -> El {
    use super::widgets::LOCK;
    let p = v.p;
    let open = v.s.active.as_deref().is_some_and(|a| v.s.note_unlocked(a));
    let help = if open { "Lock now. The note needs its password again." } else { "Lock with a password" };
    row()
        .key("fmt-lock")
        .size(28.0, 28.0)
        .center()
        .radius(6.0)
        .hover(p.hover)
        .hover_group()
        .role("button")
        .label(help)
        .help(help)
        .on_press_tx(|s: &mut App, tx| s.ask_lock(tx))
        .child(icon(&LOCK, 16.0).color(if open { p.acc } else { p.mute }).group_color(if open { p.acc } else { p.ink }))
}

/// The toolbar's pin: it pins the note to the top of the tree, or unpins it. The accent marks a
/// pinned note.
fn pin_button(v: V) -> El {
    use super::widgets::PIN;
    let p = v.p;
    let on = v.s.active.as_deref().is_some_and(|a| v.s.is_pinned(a));
    let help = if on { "Unpin" } else { "Pin to the top of the tree" };
    row()
        .key("fmt-pin")
        .size(28.0, 28.0)
        .center()
        .radius(6.0)
        .hover(p.hover)
        .hover_group()
        .role("button")
        .label(help)
        .help(help)
        .on_press(|s: &mut App| {
            if let Some(a) = s.active.clone() {
                s.toggle_pin(&a);
            }
        })
        .child(icon(&PIN, 16.0).color(if on { p.acc } else { p.mute }).group_color(if on { p.acc } else { p.ink }))
}

/// The formatting toolbar's place: a band across the bottom of the note that centres the
/// toolbar. The band takes no presses, so a click beside the toolbar reaches the text under it.
fn toolbar_dock(v: V) -> El {
    row().key("toolbar-dock").pin(Some(0.0), None, Some(0.0), Some(TOOLBAR_BOTTOM)).justify_center().child(toolbar(v))
}

/// The formatting toolbar: a pill that floats over the bottom of the note. The note scrolls
/// under it.
fn toolbar(v: V) -> El {
    use super::widgets::{BOLD, BULLETS, CODE, HEADING, IMAGE, ITALIC, LINK, NUMBERS, QUOTE, STRIKE, TASK, WIKI, icon_button};
    use crate::format::Fmt;
    let p = v.p;
    let k = crate::keys::k;
    let btn = |key: &str, glyph: &'static rune::Icon, help: String, f: Fmt| icon_button(p, key, glyph, 16.0, &help).on_press_tx(move |s: &mut App, tx| s.format(f, tx));
    let sep = |key: &str| el().key(key).w(1.0).h(16.0).mx(5.0).fill(p.line);
    // No border: a shadow lifts the toolbar off the page. On a dark page a shadow hardly shows,
    // so the toolbar is a little lighter than the page there.
    let (fill, near, far) = if p.dark { (p.bg.mix(p.ink, 0.05), 0.45, 0.55) } else { (p.bg, 0.08, 0.14) };
    row()
        .key("toolbar")
        .h(TOOLBAR_H)
        .px(4.0)
        .gap(2.0)
        .center_y()
        .radius(12.0)
        .fill(fill.alpha(0.92))
        .backdrop_blur(12.0)
        .box_shadow(rune::Shadow { x: 0.0, y: 1.0, blur: 3.0, spread: 0.0, color: Color::BLACK.alpha(near), inset: false })
        .box_shadow(rune::Shadow { x: 0.0, y: 8.0, blur: 28.0, spread: -6.0, color: Color::BLACK.alpha(far), inset: false })
        // A press between the buttons stays on the toolbar. The column under it would move
        // the caret to the end of the note.
        .on_press(|_: &mut App| {})
        .role("toolbar")
        .label("Formatting")
        .child(btn("fmt-bold", &BOLD, format!("Bold ({})", k(&["Mod", "B"])), Fmt::Bold))
        .child(btn("fmt-italic", &ITALIC, format!("Italic ({})", k(&["Mod", "I"])), Fmt::Italic))
        .child(btn("fmt-strike", &STRIKE, "Strikethrough".into(), Fmt::Strike))
        .child(btn("fmt-heading", &HEADING, "Heading".into(), Fmt::Heading))
        .child(sep("sep-1"))
        .child(btn("fmt-link", &LINK, format!("Link ({})", k(&["Mod", "K"])), Fmt::Link))
        .child(btn("fmt-wiki", &WIKI, "Note link".into(), Fmt::Wiki))
        .child(sep("sep-2"))
        .child(btn("fmt-bullets", &BULLETS, "Bulleted list".into(), Fmt::Bullet))
        .child(btn("fmt-numbers", &NUMBERS, "Numbered list".into(), Fmt::Number))
        .child(btn("fmt-task", &TASK, "Task".into(), Fmt::Task))
        .child(btn("fmt-quote", &QUOTE, "Quote".into(), Fmt::Quote))
        .child(btn("fmt-code", &CODE, "Code".into(), Fmt::Code))
        .child(sep("sep-3"))
        .child(icon_button(p, "fmt-image", &IMAGE, 16.0, &format!("Insert an image ({})", k(&["Mod", "Shift", "I"]))).on_press_tx(|s: &mut App, tx| s.pick_image(tx)))
        // A note window leaves pins and locks to the main window.
        .child(if v.s.lean.is_none() { row().key("note-tools").gap(2.0).center_y().child(sep("sep-4")).child(pin_button(v)).child(lock_button(v)) } else { el() })
}

fn crumb(v: V) -> El {
    let p = v.p;
    let s = v.s;
    let active = s.active.clone().unwrap_or_default();
    if s.lean.is_some() {
        // A note window can't rename: the main window owns the folder.
        return static_crumb(v, &active, "crumb");
    }
    let dir = md::dir(&active).to_string();
    let style = mono(12.0, 400.0);
    let mut r = row().key("crumb").h(24.0).mb(30.0).gap(6.0).center_y();
    if !dir.is_empty() {
        r = r.child(label(dir, style, p.mute)).child(label("/", style, p.mark));
    }
    match &s.renaming {
        None => r.child(
            row()
                .key("crumb-file")
                .px(5.0)
                .py(2.0)
                .mx(-5.0)
                .radius(5.0)
                .hover(p.hover)
                .role("button")
                .label("Rename note")
                .help("Rename note")
                .cursor(rune::Cursor::Text)
                .on_press_tx(|s: &mut App, tx| s.start_rename(tx))
                .child(label(md::base(&active), style, p.ink2))
                .child(label(".md", style, p.mark)),
        ),
        Some(edit) => {
            let w = (edit.text.chars().count() as f32 + 1.0).max(8.0) * 12.0 * 0.6 + 10.0;
            r.child(
                row()
                    .key("rename-row")
                    .center_y()
                    .child(
                        el().key("rename-box").w(w).px(5.0).py(2.0).mx(-5.0).radius(5.0).fill(p.soft).child(
                            text_input(edit, style)
                                .key("rename")
                                .single_line()
                                .w_full()
                                .color(p.ink)
                                .caret(p.ink, p.sel)
                                .on_edit(|s: &mut App| s.renaming.get_or_insert_with(|| TextEdit::new("")))
                                .on_submit(|s: &mut App, tx| s.commit_rename(tx))
                                .on_input_key(|s: &mut App, ev: &KeyEvent, tx: &mut Tx| {
                                    if ev.key == Key::Escape {
                                        s.cancel_rename(tx);
                                        return true;
                                    }
                                    false
                                })
                                .on_window_focus(|_s: &mut App, _f, _tx| {}),
                        ),
                    )
                    .child(label(".md", style, p.mark).ml(5.0)),
            )
        }
    }
}

fn source(v: V) -> El {
    let p = v.p;
    let s = v.s;
    let doc = s.active_text();
    let style = ed_style(v);
    let edit = s.active.as_ref().and_then(|a| s.edits.get(a));
    let caret = edit.map_or(0, |e| e.focus.min(doc.len()));
    let dim = if v.focus() && s.settings().dim_focus { Some(md::para_range(doc, caret)) } else { None };
    let hits = s.find_matches();
    let cur = s.find.as_ref().and_then(|f| f.cur);
    let mirror = mirror(v, doc, "", dim, &hits, cur);

    let empty = TextEdit::new("");
    let input = text_input(edit.unwrap_or(&empty), style)
        .key("editor")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
        .color(Color::TRANSPARENT)
        .caret(p.acc, p.sel_editor)
        .placeholder("Start writing…", theme::hex("#9a9184"))
        .spellcheck(s.settings().spellcheck)
        .label("Note")
        .on_edit(|s: &mut App| s.cur_edit())
        .on_input_change(|s: &mut App, tx: &mut Tx| {
            s.after_edit();
            typewriter(s, tx);
        })
        .on_input_paste(|s: &mut App, content, tx| s.paste_content(content, tx))
        .on_input_key(on_editor_key);
    el().key("source").w_full().child(mirror).child(input)
}

/// The highlighted lines under the editor, or a whole note in the other pane. Keys start with
/// `prefix`. `hits` are find's matches, `cur` the one selected.
fn mirror(v: V, doc: &str, prefix: &str, dim: Option<(usize, usize)>, hits: &[(usize, usize)], cur: Option<usize>) -> El {
    let p = v.p;
    let s = v.s;
    let style = ed_style(v);
    let mut mirror = col().key(format!("{prefix}hl")).w_full().a11y_hidden();
    let mut fence = false;
    // Each line is cached: typing inside a line builds only that line again.
    let theme_id = (v.dark, s.settings().font.clone(), s.settings().size, s.names_gen);
    let (dim_hit, cur_hit) = (p.acc.alpha(0.2), p.acc.alpha(0.45));
    let mut start = 0;
    let mut h = 0;
    for (i, ln) in doc.split('\n').enumerate() {
        let is_f = md::is_fence(ln);
        let in_code = is_f || fence;
        let opacity = dim.map(|(a, b)| if i < a || i > b { 0.28f32 } else { 1.0 });
        // Find's matches on this line, in line bytes. A match across lines marks each part.
        let end = start + ln.len();
        while h < hits.len() && hits[h].1 <= start {
            h += 1;
        }
        let mut marks: Vec<(usize, usize, bool)> = Vec::new();
        let mut j = h;
        while j < hits.len() && hits[j].0 <= end {
            let (a, b) = (hits[j].0.max(start), hits[j].1.min(end));
            if b > a {
                marks.push((a - start, b - start, cur == Some(j)));
            }
            j += 1;
        }
        start = end + 1;
        let version = (ln, in_code, is_f, opacity.map(f32::to_bits), &theme_id, &marks);
        let names = &s.names;
        mirror = mirror.child(rune::memo(format!("{prefix}ln-{i}"), version, || {
            let mut line_el;
            let found = marks.iter().map(|&(a, b, c)| Span { bg: Some(if c { cur_hit } else { dim_hit }), ..Span::new(a, b) });
            if in_code {
                let mut spans = if is_f { vec![Span { color: Some(p.mark), ..Span::new(0, ln.len()) }] } else { Vec::new() };
                spans.extend(found);
                line_el = rich(if ln.is_empty() { " ".to_string() } else { ln.to_string() }, spans).fill(p.code).color(p.ink2);
            } else {
                let (mut spans, strikes) = hl_line(p, ln, names);
                spans.extend(found);
                line_el = rich(if ln.is_empty() { " ".to_string() } else { ln.to_string() }, spans).color(p.ink).span_radius(3.0);
                for (a, b, c) in strikes {
                    line_el = line_el.strike(a, b, Some(c));
                }
            }
            line_el = line_el.font(style).wrap().w_full();
            if let Some(o) = opacity {
                line_el = line_el.opacity(o).opacity_motion(rune::Spring::new(0.2, 0.0));
            }
            line_el
        }));
        if is_f {
            fence = !fence;
        }
    }
    mirror
}

/// In focus mode, keeps the line being typed near the middle of the window.
fn typewriter(s: &mut App, tx: &mut Tx) {
    if !(s.focus && s.settings().typewriter) {
        return;
    }
    let text = s.active_text().to_string();
    let caret = s.cur_edit().focus.min(text.len());
    let (line, _) = md::line_col(&text, caret);
    let lh = s.settings().size as f32 * 1.75;
    let vh = s.view_h.get();
    tx.scroll_key_to(format!("ln-{line}"), (vh * 0.42 - lh / 2.0).max(0.0));
}

/// The `[[link]]` around the caret, if any.
pub fn link_at_caret(text: &str, caret: usize) -> Option<String> {
    let (_, ls) = md::line_col(text, caret);
    let le = text[ls..].find('\n').map_or(text.len(), |n| ls + n);
    let line = &text[ls..le];
    let col = caret - ls;
    for (t, r) in md::inline_tokens(line) {
        // `[[cat.png]]` names an image, not a note, so there is no note to open or make.
        if t == Tok::Wiki && col >= r.start && col <= r.end && !md::is_image(&md::link_target(&line[r.start + 2..r.end - 2])) {
            return Some(md::link_target(&line[r.start + 2..r.end - 2]).to_string());
        }
    }
    None
}

pub fn follow_link_at_caret(s: &mut App, tx: &mut Tx) {
    let text = s.active_text().to_string();
    let caret = s.cur_edit().focus.min(text.len());
    if let Some(t) = link_at_caret(&text, caret) {
        s.open_link(&t, tx);
    }
}

fn on_editor_key(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    // A caret move that follows a key is not a click (see `App::check_link_click`).
    s.key_moved = true;
    let m = ev.mods;
    match &ev.key {
        // The field would keep Escape, so focus mode and open menus take it here.
        Key::Escape => s.escape(tx),
        Key::Command(c) if (*c == 'b' || *c == 'i') && !m.shift && !m.alt => {
            let mk = if *c == 'b' { "**" } else { "*" };
            let e = s.cur_edit();
            let (a, b) = (e.anchor.min(e.focus), e.anchor.max(e.focus));
            let sel = e.text[a..b].to_string();
            e.insert(&format!("{mk}{sel}{mk}"));
            e.set_selection(a + mk.len(), a + mk.len() + sel.len());
            s.after_edit();
            true
        }
        Key::Enter if m.cmd => {
            follow_link_at_caret(s, tx);
            true
        }
        // On a list item Tab nests it and Shift+Tab moves it out. Elsewhere Tab types two spaces.
        Key::Tab if !m.cmd && !m.alt => {
            let e = s.cur_edit();
            match crate::format::indent(&e.text, e.anchor, e.focus, m.shift) {
                Some(ed) => replace(s, ed),
                None if !m.shift => {
                    s.cur_edit().insert("  ");
                    s.after_edit();
                }
                None => {}
            }
            true
        }
        Key::Enter if !m.shift && !m.alt => {
            let cont = s.settings().continue_lists;
            let e = s.cur_edit();
            if e.anchor == e.focus && cont {
                let pos = e.focus;
                let ls = e.text[..pos].rfind('\n').map_or(0, |i| i + 1);
                let le = e.text[pos..].find('\n').map_or(e.text.len(), |i| pos + i);
                let line = e.text[ls..le].to_string();
                if let Some(l) = md::list_line(&line) {
                    let text_at = match &l.task {
                        Some(t) => t.end + line[t.end..].len() - line[t.end..].trim_start().len(),
                        None => l.after,
                    };
                    if pos - ls >= text_at {
                        if line[text_at..].trim().is_empty() {
                            // An empty nested item moves out a level. At the edge it ends the list.
                            if l.indent > 0
                                && let Some(ed) = crate::format::indent(&e.text, pos, pos, true)
                            {
                                replace(s, ed);
                                typewriter(s, tx);
                                return true;
                            }
                            e.set_selection(ls, le);
                            e.insert("");
                        } else {
                            let mk = match l.number {
                                Some((n, d)) => format!("{}{d}", n + 1),
                                None => line[l.marker.clone()].to_string(),
                            };
                            let spaces = &line[l.marker.end..l.after];
                            let task = if l.task.is_some() { "[ ] " } else { "" };
                            e.insert(&format!("\n{}{mk}{spaces}{task}", &line[..l.indent]));
                        }
                        s.after_edit();
                        typewriter(s, tx);
                        return true;
                    }
                }
            }
            s.cur_edit().insert("\n");
            s.after_edit();
            typewriter(s, tx);
            true
        }
        _ => false,
    }
}

/// One edit to the note, as one undo step.
fn replace(s: &mut App, ed: crate::format::Edit) {
    let e = s.cur_edit();
    e.undo_group(|e| {
        e.set_selection(ed.start, ed.end);
        e.insert(&ed.insert);
        e.set_selection(ed.sel.0, ed.sel.1);
    });
    s.after_edit();
}

/// Source highlighting for one line: spans and strikethrough ranges, as the design's `hlLine`.
pub fn hl_line(p: &Pal, ln: &str, names: &md::Names) -> (Vec<Span>, Vec<(usize, usize, Color)>) {
    let mut spans = Vec::new();
    let mut strikes = Vec::new();
    let mk = |a: usize, b: usize| Span { color: Some(p.mark), weight: Some(400.0), ..Span::new(a, b) };
    if let Some((_, start)) = md::heading_line(ln) {
        spans.push(Span { color: Some(p.acc.alpha(0.6)), ..Span::new(0, start) });
        let base = Base { weight: Some(600.0), color: Some(p.ink), italic: false, strike: false };
        hl_inline(p, ln, start, base, names, &mut spans, &mut strikes);
        return (spans, strikes);
    }
    if md::is_hr(ln) {
        spans.push(mk(0, ln.len()));
        return (spans, strikes);
    }
    if let Some(q) = md::quote_prefix(ln) {
        spans.push(Span { color: Some(p.acc), ..Span::new(0, q) });
        hl_inline(p, ln, q, Base { weight: None, color: Some(p.ink2), italic: true, strike: false }, names, &mut spans, &mut strikes);
        return (spans, strikes);
    }
    if let Some(l) = md::list_line(ln) {
        spans.push(Span { color: Some(p.acc), ..Span::new(l.marker.start, l.after) });
        let mut base = Base::default();
        let from = match &l.task {
            Some(t) => {
                // The design's box match takes exactly one whitespace character after `]`.
                spans.push(Span { color: Some(if l.done { p.mute } else { p.acc }), ..Span::new(t.start, t.end) });
                if l.done {
                    base = Base { weight: None, color: Some(p.mute), italic: false, strike: true };
                }
                t.end
            }
            None => l.after,
        };
        hl_inline(p, ln, from, base, names, &mut spans, &mut strikes);
        return (spans, strikes);
    }
    hl_inline(p, ln, 0, Base::default(), names, &mut spans, &mut strikes);
    (spans, strikes)
}

#[derive(Clone, Copy, Default)]
struct Base {
    weight: Option<f32>,
    color: Option<Color>,
    italic: bool,
    strike: bool,
}

fn hl_inline(p: &Pal, ln: &str, from: usize, base: Base, names: &md::Names, spans: &mut Vec<Span>, strikes: &mut Vec<(usize, usize, Color)>) {
    let s = &ln[from..];
    let plain = |a: usize, b: usize, spans: &mut Vec<Span>, strikes: &mut Vec<(usize, usize, Color)>| {
        if a >= b {
            return;
        }
        spans.push(Span { weight: base.weight, color: base.color, italic: base.italic, ..Span::new(from + a, from + b) });
        if base.strike {
            strikes.push((from + a, from + b, p.mark));
        }
    };
    let styled = |a: usize, b: usize, sp: Span, spans: &mut Vec<Span>, strikes: &mut Vec<(usize, usize, Color)>| {
        if a >= b {
            return;
        }
        let mut x = Span { start: (from + a) as u32, end: (from + b) as u32, ..sp };
        if x.weight.is_none() {
            x.weight = base.weight;
        }
        if x.color.is_none() {
            x.color = base.color;
        }
        x.italic = x.italic || base.italic;
        spans.push(x);
        if base.strike {
            strikes.push((from + a, from + b, p.mark));
        }
    };
    let mk = |a: usize, b: usize, spans: &mut Vec<Span>| {
        if a < b {
            spans.push(Span { color: Some(p.mark), weight: Some(400.0), ..Span::new(from + a, from + b) });
        }
    };
    let mut last = 0;
    for (t, r) in md::inline_tokens(s) {
        plain(last, r.start, spans, strikes);
        let (a, b) = (r.start, r.end);
        match t {
            Tok::Code => {
                mk(a, a + 1, spans);
                styled(a + 1, b - 1, Span { bg: Some(p.code), color: Some(p.ink2), ..Span::default() }, spans, strikes);
                mk(b - 1, b, spans);
            }
            Tok::Wiki => {
                let inner = &s[a + 2..b - 2];
                let image = md::is_image(&md::link_target(inner));
                let ok = names.resolve(&md::link_target(inner)).is_some();
                mk(a, a + 2, spans);
                // An image embed is no note link: it gets the second ink and no underline.
                let sp = if image {
                    Span { color: Some(p.ink2), ..Span::default() }
                } else {
                    Span { color: Some(if ok { p.acc } else { p.mute }), underline: Some(if ok { p.link_line } else { p.mark }), ..Span::default() }
                };
                styled(a + 2, b - 2, sp, spans, strikes);
                mk(b - 2, b, spans);
            }
            Tok::Bold => {
                mk(a, a + 2, spans);
                styled(a + 2, b - 2, Span { weight: Some(600.0), color: Some(p.ink), ..Span::default() }, spans, strikes);
                mk(b - 2, b, spans);
            }
            Tok::Italic => {
                mk(a, a + 1, spans);
                styled(a + 1, b - 1, Span { italic: true, ..Span::default() }, spans, strikes);
                mk(b - 1, b, spans);
            }
            Tok::Strike => {
                mk(a, a + 2, spans);
                styled(a + 2, b - 2, Span { color: Some(p.mute), ..Span::default() }, spans, strikes);
                strikes.push((from + a + 2, from + b - 2, p.mute));
                mk(b - 2, b, spans);
            }
            Tok::Link => {
                let j = s[a..b].find("](").map_or(b, |j| a + j);
                mk(a, a + 1, spans);
                styled(a + 1, j, Span { color: Some(p.acc), ..Span::default() }, spans, strikes);
                mk(j, b, spans);
            }
            Tok::Tag => {
                styled(a, b, Span { color: Some(p.acc), bg: Some(p.soft), ..Span::default() }, spans, strikes);
            }
        }
        last = b;
    }
    plain(last, s.len(), spans, strikes);
}

fn focus_hint(v: V) -> El {
    let p = v.p;
    let wc = md::words(v.s.active_text());
    row()
        .key("focus-hint")
        .pin(Some(0.0), None, Some(0.0), Some(14.0))
        .justify_center()
        .inert()
        .child(super::widgets::padded(format!("{} words · esc to leave focus", group(wc)), mono(11.5, 400.0), p.mute, 10.0, 4.0, 0.0).radius(6.0).fill(p.bg))
}

/// 1234 as "1,234", as `toLocaleString` writes it in English.
pub fn group(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn no_tab(v: V) -> El {
    let p = v.p;
    let hints: [(&str, &[&str], fn(&mut App, &mut Tx)); 4] = [
        ("Open a note", &["Mod", "O"], |s, tx| s.open_palette(crate::app::PalMode::Files, tx)),
        ("New note", &["Mod", "N"], |s, tx| s.new_note(None, tx)),
        ("All commands", &["Mod", "P"], |s, tx| s.open_palette(crate::app::PalMode::Commands, tx)),
        ("Search", &["Mod", "Shift", "F"], |s, tx| s.open_search(tx)),
    ];
    let mut c = col().key("hints").w(260.0).gap(12.0);
    for (i, (l, keys, run)) in hints.into_iter().enumerate() {
        c = c.child(
            row()
                .key(format!("hint-{i}"))
                .gap(24.0)
                .justify_between()
                .hover_group()
                .role("button")
                .on_press_tx(move |s: &mut App, tx| run(s, tx))
                .child(label(l, mono(12.5, 400.0), p.ink2).group_color(p.acc))
                .child(label(k(keys), mono(12.5, 400.0), p.mute)),
        );
    }
    c
}

fn empty_vault(v: V) -> El {
    let p = v.p;
    let path = v.s.vault.as_ref().map_or(String::new(), |x| crate::store::tilde(&x.path));
    col()
        .key("empty-vault")
        .max_w(380.0)
        .gap(14.0)
        .items_start()
        .child(label("Nothing here yet.", theme::serif(32.0, 400.0).lh(32.0 * 1.15), p.ink))
        .child(text(md::pretty(&format!("Notes you write here are saved as plain .md files in {path}."))).font(sans(14.0, 400.0).lh(14.0 * 1.6)).color(p.ink2).wrap())
        .child(
            row()
                .key("first-note")
                .h(34.0)
                .mt(6.0)
                .px(14.0)
                .gap(12.0)
                .center_y()
                .radius(8.0)
                .fill(p.button)
                .hover(p.button_hover)
                .role("button")
                .on_press_tx(|s: &mut App, tx| s.new_note(None, tx))
                .child(label("Create your first note", sans(13.5, 500.0), p.on_acc))
                .child(label(k(&["Mod", "N"]), mono(11.0, 400.0), p.on_acc.alpha(0.8))),
        )
}
