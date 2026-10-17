//! `InlineHighlighter`: live preview for one text block. It parses the
//! block's markdown with `md::inline` and formats it in place: strong text
//! bold, links in the link colour, and so on. Syntax markers take no room
//! unless the caret touches their span; then they show in a muted colour,
//! as in Typora. Moving the caret only changes formats, never the text, so
//! undo, input methods and selection are unaffected.
//!
//! Inline images and formulas (objects) are drawn over the text by QML.
//! Once QML reports an object's size, its source is hidden but for one
//! character set in a strut font (`crate::strut`) that leaves room for it;
//! `objectAnchor` says where that is. With the caret in it, an object shows
//! as source again.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qpointf.h");
        type QPointF = cxx_qt_lib::QPointF;
        include!("cxx-qt-lib/qrectf.h");
        type QRectF = cxx_qt_lib::QRectF;
    }

    unsafe extern "C++Qt" {
        include!("typo_glue.h");
        #[qobject]
        type TypoHighlighterBase;
        #[qobject]
        type QQuickTextDocument;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = TypoHighlighterBase]
        /// Line height in pixels, like CSS `line-height` (0 leaves Qt's).
        #[qproperty(f64, line_height, cxx_name = "lineHeight")]
        /// The block's font size in pixels; inline code is 0.9 of it.
        #[qproperty(f64, font_size, cxx_name = "fontSize")]
        /// The block is one image that has loaded: away from the caret it
        /// shows instead of its source.
        #[qproperty(bool, image_shown, cxx_name = "imageShown")]
        /// The block's inline images and formulas as JSON, in order:
        /// `[{"kind":"image","src":"a.png"},{"kind":"math","src":"x^2"}]`.
        #[qproperty(QString, objects)]
        /// Some object is drawn in the text, so its lines may grow.
        #[qproperty(bool, placing)]
        /// The code spans' text as JSON `[[start, end], …]` in UTF-16, for
        /// the boxes the delegate draws around them.
        #[qproperty(QString, boxes)]
        /// Room before the first line, in pixels (a footnote's label).
        #[qproperty(f64, first_indent, cxx_name = "firstIndent")]
        /// The block opens an alert: its `[!NOTE]` is hidden away from the
        /// caret, where the delegate draws the title.
        #[qproperty(bool, alert_title, cxx_name = "alertTitle")]
        /// The block is a link reference definition, drawn as Typora's
        /// `md-def-link` rather than as inline markdown.
        #[qproperty(bool, link_def, cxx_name = "linkDef")]
        /// Find's matches in the block, as JSON `[[start, end], …]` in
        /// UTF-16; they show over the text but not over hidden markers.
        #[qproperty(QString, hits)]
        /// The document's `Editor.docId`, for its links and options.
        #[qproperty(i32, doc)]
        /// The markdown the text box shows (`Editor.boxed`), where it
        /// differs: a `<br>`'s `>` is a line break in the box.
        #[qproperty(QString, source)]
        /// Where the `>` of each `<br>` that shows as source would be, as
        /// JSON `[pos, …]` in UTF-16: it is a line break, so the delegate
        /// draws it.
        #[qproperty(QString, breaks)]
        type InlineHighlighter = super::InlineHighlighterRust;

        /// Formats changed: objects may have moved.
        #[qsignal]
        fn formatted(self: Pin<&mut Self>);

        #[cxx_override]
        #[cxx_name = "highlightText"]
        fn highlight_text(self: Pin<&mut Self>, text: &QString);

        #[inherit]
        #[cxx_name = "applyFormat"]
        fn apply_format(self: Pin<&mut Self>, start: i32, count: i32, flags: u32, px: f64, fg: u32, bg: u32);

        #[inherit]
        #[cxx_name = "addBackground"]
        fn add_background(self: Pin<&mut Self>, start: i32, count: i32, bg: u32);

        #[inherit]
        #[cxx_name = "addSpacing"]
        fn add_spacing(self: Pin<&mut Self>, start: i32, count: i32, px: f64);

        #[inherit]
        #[cxx_name = "blockPosition"]
        fn block_position(self: &Self) -> i32;

        #[inherit]
        #[cxx_name = "blockNumber"]
        fn block_number(self: &Self) -> i32;

        #[inherit]
        #[cxx_name = "documentText"]
        fn document_text(self: &Self) -> QString;

        #[inherit]
        #[cxx_name = "attachQuickDocument"]
        unsafe fn attach_quick_document(self: Pin<&mut Self>, doc: *mut QQuickTextDocument);

        #[inherit]
        #[cxx_name = "rehighlightAll"]
        fn rehighlight_all(self: Pin<&mut Self>);

        #[inherit]
        #[cxx_name = "applyLineHeight"]
        fn apply_line_height(self: Pin<&mut Self>, pixels: f64, minimum: bool, first_indent: f64);

        #[inherit]
        #[cxx_name = "applyStrut"]
        fn apply_strut(self: Pin<&mut Self>, start: i32, step: i32, pixel_size: f64, width: f64, keep_advance: bool);

        #[inherit]
        #[cxx_name = "anchorAt"]
        fn anchor_at(self: &Self, pos: i32) -> QPointF;

        #[inherit]
        #[cxx_name = "dimLine"]
        fn dim_line(self: Pin<&mut Self>, fg: u32);

        #[inherit]
        #[cxx_name = "rehighlightLine"]
        fn rehighlight_line(self: Pin<&mut Self>, line: i32);

        /// Focus mode: grey every line (`blur`) but line `keep`, -1 for
        /// none; `blur` false leaves the colours alone.
        #[qinvokable]
        fn dim(self: Pin<&mut Self>, blur: bool, keep: i32);

        #[qinvokable]
        unsafe fn attach(self: Pin<&mut Self>, doc: *mut QQuickTextDocument);

        /// Re-parse and repaint everything (theme, options or links changed).
        #[qinvokable]
        fn refresh(self: Pin<&mut Self>);

        /// `source` changed: repaint if the text reads differently now.
        #[qinvokable]
        fn reread(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "fixLineHeight"]
        fn fix_line_height(self: Pin<&mut Self>);

        /// The caret moved to `pos` (UTF-16), or left the block (-1).
        #[qinvokable]
        #[cxx_name = "caretMoved"]
        fn caret_moved(self: Pin<&mut Self>, pos: i32);

        /// Where object `index` is drawn: the x of its place in the text
        /// and the y of the baseline there, or NaN while it shows as source.
        #[qinvokable]
        #[cxx_name = "objectAnchor"]
        fn object_anchor(self: &Self, index: i32) -> QPointF;

        /// While object `index` is edited and stays drawn after its source,
        /// the source's last character, which QML draws before it; else
        /// empty.
        #[qinvokable]
        #[cxx_name = "objectTail"]
        fn object_tail(self: &Self, index: i32) -> QString;

        /// The x of object `index`'s source in the text and the y of the
        /// baseline there, for a preview under it.
        #[qinvokable]
        #[cxx_name = "sourceAnchor"]
        fn source_anchor(self: &Self, index: i32) -> QPointF;

        /// The y of the baseline of the line `pos` is on, NaN before the
        /// text is laid out.
        #[qinvokable]
        #[cxx_name = "baselineAt"]
        fn baseline_at(self: &Self, pos: i32) -> f64;

        /// The object `kind` `src` is `width` wide and reaches `above` over
        /// the baseline and `below` under it, in pixels.
        #[qinvokable]
        #[cxx_name = "setObjectSize"]
        fn set_object_size(self: Pin<&mut Self>, key: &QString, width: f64, above: f64, below: f64);
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = TypoHighlighterBase]
        /// The fence's info string; its first word names the language.
        #[qproperty(QString, language)]
        /// Find's matches, as the inline highlighter's `hits`.
        #[qproperty(QString, hits)]
        /// A broken diagram's code at fault, the same way.
        #[qproperty(QString, errors)]
        type CodeHighlighter = super::CodeHighlighterRust;

        #[cxx_override]
        #[cxx_name = "highlightText"]
        fn highlight_text(self: Pin<&mut Self>, text: &QString);

        #[inherit]
        #[cxx_name = "applyFormat"]
        fn apply_format(self: Pin<&mut Self>, start: i32, count: i32, flags: u32, px: f64, fg: u32, bg: u32);

        #[inherit]
        #[cxx_name = "addBackground"]
        fn add_background(self: Pin<&mut Self>, start: i32, count: i32, bg: u32);

        #[inherit]
        #[cxx_name = "addWave"]
        fn add_wave(self: Pin<&mut Self>, start: i32, count: i32, color: u32);

        #[inherit]
        #[cxx_name = "blockNumber"]
        fn block_number(self: &Self) -> i32;

        #[inherit]
        #[cxx_name = "setBlockState"]
        fn set_block_state(self: Pin<&mut Self>, state: i32);

        #[inherit]
        #[cxx_name = "documentText"]
        fn document_text(self: &Self) -> QString;

        #[inherit]
        #[cxx_name = "attachQuickDocument"]
        unsafe fn attach_quick_document(self: Pin<&mut Self>, doc: *mut QQuickTextDocument);

        #[inherit]
        #[cxx_name = "rehighlightAll"]
        fn rehighlight_all(self: Pin<&mut Self>);

        #[inherit]
        #[cxx_name = "dimLine"]
        fn dim_line(self: Pin<&mut Self>, fg: u32);

        #[inherit]
        #[cxx_name = "rehighlightLine"]
        fn rehighlight_line(self: Pin<&mut Self>, line: i32);

        /// Focus mode: grey every line (`blur`) but line `keep`, -1 for
        /// none; `blur` false leaves the colours alone.
        #[qinvokable]
        fn dim(self: Pin<&mut Self>, blur: bool, keep: i32);

        #[qinvokable]
        unsafe fn attach(self: Pin<&mut Self>, doc: *mut QQuickTextDocument);

        /// Repaint everything (the language or the theme changed).
        #[qinvokable]
        fn refresh(self: Pin<&mut Self>);

        #[inherit]
        #[cxx_name = "applyLineHeight"]
        fn apply_line_height(self: Pin<&mut Self>, pixels: f64, minimum: bool, first_indent: f64);

        /// Give every line `pixels` of height: lines typed later take it
        /// from the line they were split from.
        #[qinvokable]
        #[cxx_name = "fixLineHeight"]
        fn fix_line_height(self: Pin<&mut Self>, pixels: f64);

        #[inherit]
        #[cxx_name = "docLineNumber"]
        fn doc_line_number(self: &Self, pos: i32) -> i32;

        #[inherit]
        #[cxx_name = "docLinePosition"]
        fn doc_line_position(self: &Self, line: i32) -> i32;

        #[inherit]
        #[cxx_name = "docLineText"]
        fn doc_line_text(self: &Self, pos: i32) -> QString;

        /// Shift+Tab with Preferences' "Use Shift+Tab to auto indent": each
        /// line from `anchor` to `caret` takes the indent of the line
        /// above, JSON as SourceHighlighter's `indentAt`.
        #[qinvokable]
        #[cxx_name = "autoIndent"]
        fn auto_indent(self: &Self, anchor: i32, caret: i32, tab: i32) -> QString;
    }

    unsafe extern "RustQt" {
        /// Source Code Mode's colours for the whole markdown, a line at a
        /// time (src/source.rs).
        #[qobject]
        #[qml_element]
        #[base = TypoHighlighterBase]
        /// The text's pixel size, which Typora scales for the first two
        /// header levels.
        #[qproperty(f64, font_size, cxx_name = "fontSize")]
        /// Find's matches, as the inline highlighter's `hits`, in the whole
        /// text.
        #[qproperty(QString, hits)]
        /// As the inline highlighter's `doc`.
        #[qproperty(i32, doc)]
        type SourceHighlighter = super::SourceHighlighterRust;

        #[cxx_override]
        #[cxx_name = "highlightText"]
        fn highlight_text(self: Pin<&mut Self>, text: &QString);

        #[inherit]
        #[cxx_name = "applyFormat"]
        fn apply_format(self: Pin<&mut Self>, start: i32, count: i32, flags: u32, px: f64, fg: u32, bg: u32);

        #[inherit]
        #[cxx_name = "addBackground"]
        fn add_background(self: Pin<&mut Self>, start: i32, count: i32, bg: u32);

        #[inherit]
        #[cxx_name = "blockNumber"]
        fn block_number(self: &Self) -> i32;

        #[inherit]
        #[cxx_name = "blockPosition"]
        fn block_position(self: &Self) -> i32;

        #[inherit]
        #[cxx_name = "previousState"]
        fn previous_state(self: &Self) -> i32;

        #[inherit]
        #[cxx_name = "setBlockState"]
        fn set_block_state(self: Pin<&mut Self>, state: i32);

        #[inherit]
        #[cxx_name = "attachQuickDocument"]
        unsafe fn attach_quick_document(self: Pin<&mut Self>, doc: *mut QQuickTextDocument);

        #[inherit]
        #[cxx_name = "rehighlightAll"]
        fn rehighlight_all(self: Pin<&mut Self>);

        #[inherit]
        #[cxx_name = "applyLineHeight"]
        fn apply_line_height(self: Pin<&mut Self>, pixels: f64, minimum: bool, first_indent: f64);

        #[inherit]
        #[cxx_name = "clearUndo"]
        fn clear_undo(self: Pin<&mut Self>);

        #[inherit]
        #[cxx_name = "replaceRange"]
        fn replace_range_base(self: Pin<&mut Self>, start: i32, end: i32, text: &QString, join: bool);

        /// Replace `start`..`end` with `text`, as part of the last edit
        /// when `join` (one undo step for Replace All).
        #[qinvokable]
        #[cxx_name = "replaceText"]
        fn replace_text(self: Pin<&mut Self>, start: i32, end: i32, text: &QString, join: bool);

        #[inherit]
        #[cxx_name = "docLineNumber"]
        fn doc_line_number(self: &Self, pos: i32) -> i32;

        #[inherit]
        #[cxx_name = "docLineStart"]
        fn doc_line_start(self: &Self, pos: i32) -> i32;

        #[inherit]
        #[cxx_name = "docLinePosition"]
        fn doc_line_position(self: &Self, line: i32) -> i32;

        #[inherit]
        #[cxx_name = "docLineRect"]
        fn doc_line_rect(self: &Self, pos: i32) -> QRectF;

        #[inherit]
        #[cxx_name = "docLineText"]
        fn doc_line_text(self: &Self, pos: i32) -> QString;

        #[inherit]
        #[cxx_name = "docStateBefore"]
        fn doc_state_before(self: &Self, pos: i32) -> i32;

        #[inherit]
        #[cxx_name = "nextLineText"]
        fn next_line_text(self: &Self) -> QString;

        #[inherit]
        #[cxx_name = "docInkAt"]
        fn doc_ink_at(self: &Self, pos: i32) -> QString;

        /// The colour of the character at `pos`, `"#rrggbb"`.
        #[qinvokable]
        #[cxx_name = "inkAt"]
        fn ink_at(self: &Self, pos: i32) -> QString;

        /// Colour the line before the one holding `pos` again: what an
        /// underline typed under it makes of it.
        #[qinvokable]
        #[cxx_name = "restyleAbove"]
        fn restyle_above(self: Pin<&mut Self>, pos: i32);

        /// What Enter does over `start`..`end`: JSON `[from, to, text,
        /// anchor, caret]`, the edit and the selection after it.
        #[qinvokable]
        #[cxx_name = "enterAt"]
        fn enter_at(self: &Self, start: i32, end: i32, same: bool, pairs: bool, tab: i32) -> QString;

        /// Tab over a selection (`add`) or Shift+Tab, from `anchor` to
        /// `caret`: JSON as `enterAt`'s.
        #[qinvokable]
        #[cxx_name = "indentAt"]
        fn indent_at(self: &Self, anchor: i32, caret: i32, add: bool, tab: i32) -> QString;

        /// What typing `ch` between `prev` and `next` does: JSON
        /// `["type" | "skip" | "wrap" | "both", left, right]`.
        #[qinvokable]
        #[cxx_name = "pairFor"]
        fn pair_for(self: &Self, ch: &QString, prev: &QString, next: &QString, selected: bool) -> QString;

        /// Whether Backspace between `prev` and `next` deletes a pair.
        #[qinvokable]
        #[cxx_name = "pairAround"]
        fn pair_around(self: &Self, prev: &QString, next: &QString) -> bool;

        /// The line (from 0) holding text position `pos`.
        #[qinvokable]
        #[cxx_name = "lineNumberAt"]
        fn line_number_at(self: &Self, pos: i32) -> i32;

        /// Where the line holding `pos` starts.
        #[qinvokable]
        #[cxx_name = "lineStartAt"]
        fn line_start_at(self: &Self, pos: i32) -> i32;

        /// Where line `line` starts (the last line's start past the end).
        #[qinvokable]
        #[cxx_name = "linePosition"]
        fn line_position(self: &Self, line: i32) -> i32;

        /// The box of the line holding `pos`, all its wrapped rows.
        #[qinvokable]
        #[cxx_name = "lineRect"]
        fn line_rect(self: &Self, pos: i32) -> QRectF;

        #[inherit]
        #[cxx_name = "dimLine"]
        fn dim_line(self: Pin<&mut Self>, fg: u32);

        #[inherit]
        #[cxx_name = "rehighlightLine"]
        fn rehighlight_line(self: Pin<&mut Self>, line: i32);

        /// Focus mode: grey every line (`blur`) but line `keep`, -1 for
        /// none; `blur` false leaves the colours alone.
        #[qinvokable]
        fn dim(self: Pin<&mut Self>, blur: bool, keep: i32);

        #[qinvokable]
        unsafe fn attach(self: Pin<&mut Self>, doc: *mut QQuickTextDocument);

        /// Repaint everything (the theme or the size changed).
        #[qinvokable]
        fn refresh(self: Pin<&mut Self>);

        /// Give every line Typora's 1.6 line height, letting larger
        /// headers grow it, and start the text's undo afresh: the change
        /// isn't the user's. Lines typed later take it from the line they
        /// were split from.
        #[qinvokable]
        #[cxx_name = "fixLineHeight"]
        fn fix_line_height(self: Pin<&mut Self>);
    }
}

use core::pin::Pin;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QPointF, QString};

pub use super::code_highlighter::CodeHighlighterRust;
pub use super::source_highlighter::SourceHighlighterRust;
use super::editor::{byte_of, json_str, utf16_of};
use super::shared;
use crate::md::inline::{self, Kind, Node};
use crate::md::query;

pub const BOLD: u32 = 1;
pub const ITALIC: u32 = 2;
pub const UNDERLINE: u32 = 4;
pub const STRIKE: u32 = 8;
pub const MONO: u32 = 16;
pub const HIDDEN: u32 = 32;
pub const SUPER: u32 = 64;
pub const SUB: u32 = 128;
/// 0.9 of the block's font size.
pub const SMALL: u32 = 256;

/// Focus mode on one highlighter: whether it greys its lines, and the one
/// line it leaves alone (-1 for none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dim {
    pub blur: bool,
    pub keep: i32,
}

impl Default for Dim {
    fn default() -> Self {
        Dim { blur: false, keep: -1 }
    }
}

impl Dim {
    pub fn new(blur: bool, keep: i32) -> Dim {
        Dim { blur, keep: if blur { keep } else { -1 } }
    }

    /// Whether line `line` is greyed.
    pub fn greys(self, line: i32) -> bool {
        self.blur && line != self.keep
    }

    /// The lines to repaint on going to `next`: only the kept ones when
    /// just the caret's line moved, else every line (`None`).
    pub fn changed(self, next: Dim) -> Option<[i32; 2]> {
        (self.blur && next.blur).then_some([self.keep, next.keep])
    }
}

/// A theme colour, by role.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Paint {
    #[default]
    None,
    Link,
    Code,
    CodeBackground,
    Math,
    Mark,
    Marker,
    Comment,
    Footnote,
    FootnoteBackground,
    /// An alert's title, by kind (`query::ALERTS`).
    Alert(u8),
    /// A colour the document gives, as 0xAARRGGBB.
    Rgb(u32),
}

/// The room an object's carrier character makes: strut font `step` at
/// `px` pixels, spaced out to `width` sixty-fourths of a pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strut {
    pub step: u16,
    pub px: u32,
    pub width: u32,
    /// The character's own width is added: QML draws it before the object.
    pub tail: bool,
}

impl Strut {
    fn new(width: f64, above: f64, below: f64) -> Self {
        let (step, px) = crate::strut::choose(above, below);
        Strut { step, px: px.ceil() as u32, width: (width.max(0.0) * 64.0).round() as u32, tail: false }
    }
}

/// How a run of source text looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Look {
    pub flags: u32,
    pub fg: Paint,
    pub bg: Paint,
    /// Room for an object instead of text.
    pub strut: Option<Strut>,
}

const HIDE: Look = Look { flags: HIDDEN, fg: Paint::None, bg: Paint::None, strut: None };
/// Unseen text that keeps its room: transparent (alpha 0, so not "none").
const GAP: Look = Look { flags: 0, fg: Paint::Rgb(0x00ff_ffff), bg: Paint::None, strut: None };

/// The look of a node's content inside a parent that looks like `l`.
fn content_look(kind: &Kind, mut l: Look) -> Look {
    match kind {
        Kind::Strong => l.flags |= BOLD,
        Kind::Emph => l.flags |= ITALIC,
        Kind::Strike => l.flags |= STRIKE,
        Kind::Highlight => l.bg = Paint::Mark,
        Kind::Sup => l.flags |= SUPER,
        Kind::Sub => l.flags |= SUB,
        // The delegate draws the box, as Typora's `code`.
        Kind::Code => {
            l.flags |= MONO;
            l.fg = Paint::Code;
        }
        // TeX shows as it is, in its own colour, while it is edited or
        // until it is typeset.
        Kind::Math => l.fg = Paint::Math,
        // Underlined, as a browser draws Typora's `<a>`.
        Kind::Link { .. } | Kind::Autolink { .. } => {
            l.fg = Paint::Link;
            l.flags |= UNDERLINE;
        }
        // In a box, as Typora's `sup.md-footnote`.
        Kind::FootnoteRef { .. } => {
            l.flags |= SUPER;
            l.fg = Paint::Footnote;
            l.bg = Paint::FootnoteBackground;
        }
        Kind::Tag { name, color, background } => {
            if let Some(c) = color.as_deref().and_then(css_color) {
                l.fg = Paint::Rgb(c);
            }
            if let Some(c) = background.as_deref().and_then(css_color) {
                l.bg = Paint::Rgb(c);
            }
            tag_look(name, &mut l);
        }
        _ => {}
    }
    l
}

/// What an HTML tag Typora draws does to its content.
fn tag_look(name: &str, l: &mut Look) {
    match name {
        "u" | "ins" => l.flags |= UNDERLINE,
        "b" | "strong" => l.flags |= BOLD,
        "i" | "em" | "cite" | "var" | "dfn" => l.flags |= ITALIC,
        "s" | "del" | "strike" => l.flags |= STRIKE,
        "mark" => l.bg = Paint::Mark,
        "sup" => l.flags |= SUPER,
        "sub" => l.flags |= SUB,
        "code" | "kbd" | "samp" | "tt" => {
            l.flags |= MONO;
            l.bg = Paint::CodeBackground;
        }
        "a" => {
            l.fg = Paint::Link;
            l.flags |= UNDERLINE;
        }
        _ => {}
    }
}

/// A CSS colour as 0xAARRGGBB: `#rgb`, `#rrggbb`, `rgb(…)` or a common name.
fn css_color(s: &str) -> Option<u32> {
    let s = s.trim().trim_end_matches("!important").trim().to_ascii_lowercase();
    let rgb = |r: u32, g: u32, b: u32| 0xff00_0000 | r << 16 | g << 8 | b;
    if let Some(h) = s.strip_prefix('#') {
        let v = u32::from_str_radix(h, 16).ok()?;
        return match h.len() {
            3 => Some(rgb((v >> 8 & 0xf) * 17, (v >> 4 & 0xf) * 17, (v & 0xf) * 17)),
            6 => Some(0xff00_0000 | v),
            _ => None,
        };
    }
    if let Some(args) = s.strip_prefix("rgb(").or_else(|| s.strip_prefix("rgba(")).and_then(|a| a.strip_suffix(')')) {
        let parts: Vec<u32> = args.split([',', ' ', '/']).filter(|p| !p.is_empty()).take(3).filter_map(|p| p.parse::<f64>().ok()).map(|v| v.clamp(0.0, 255.0) as u32).collect();
        return (parts.len() == 3).then(|| rgb(parts[0], parts[1], parts[2]));
    }
    const NAMES: &[(&str, u32)] = &[
        ("black", 0x000000), ("white", 0xffffff), ("red", 0xff0000), ("green", 0x008000), ("blue", 0x0000ff),
        ("yellow", 0xffff00), ("orange", 0xffa500), ("purple", 0x800080), ("pink", 0xffc0cb), ("brown", 0xa52a2a),
        ("gray", 0x808080), ("grey", 0x808080), ("silver", 0xc0c0c0), ("maroon", 0x800000), ("olive", 0x808000),
        ("lime", 0x00ff00), ("aqua", 0x00ffff), ("cyan", 0x00ffff), ("teal", 0x008080), ("navy", 0x000080),
        ("fuchsia", 0xff00ff), ("magenta", 0xff00ff), ("gold", 0xffd700), ("violet", 0xee82ee), ("indigo", 0x4b0082),
        ("crimson", 0xdc143c), ("coral", 0xff7f50), ("tomato", 0xff6347), ("salmon", 0xfa8072), ("orchid", 0xda70d6),
        ("darkred", 0x8b0000), ("darkgreen", 0x006400), ("darkblue", 0x00008b), ("darkorange", 0xff8c00),
        ("darkgray", 0xa9a9a9), ("darkgrey", 0xa9a9a9), ("lightgray", 0xd3d3d3), ("lightgrey", 0xd3d3d3),
        ("lightblue", 0xadd8e6), ("lightgreen", 0x90ee90), ("skyblue", 0x87ceeb), ("steelblue", 0x4682b4),
        ("royalblue", 0x4169e1), ("deepskyblue", 0x00bfff), ("dodgerblue", 0x1e90ff), ("forestgreen", 0x228b22),
        ("seagreen", 0x2e8b57), ("limegreen", 0x32cd32), ("chocolate", 0xd2691e), ("firebrick", 0xb22222),
        ("hotpink", 0xff69b4), ("deeppink", 0xff1493), ("khaki", 0xf0e68c), ("tan", 0xd2b48c), ("beige", 0xf5f5dc),
        ("turquoise", 0x40e0d0), ("plum", 0xdda0dd), ("lavender", 0xe6e6fa), ("slategray", 0x708090),
    ];
    NAMES.iter().find(|(n, _)| *n == s).map(|(_, v)| 0xff00_0000 | v)
}

/// Nodes shown as source whatever the caret does: images, which render
/// beside their source (a paragraph of one image hides it, in `Cache`).
fn always_source(kind: &Kind) -> bool {
    matches!(kind, Kind::Image { .. })
}

fn has_markers(n: &Node) -> bool {
    n.range != n.content
}

/// Whether the caret touches `n`, which reveals its markers. Touching an
/// edge counts, so the markers appear as the caret arrives at the span.
fn touches(n: &Node, caret: Option<usize>) -> bool {
    // Smart punctuation shows its source only with the caret inside it
    // (between the dashes of `--`), so a quote just typed stays curly.
    if let Kind::Smart(_) = n.kind {
        return caret.is_some_and(|c| n.range.start < c && c < n.range.end);
    }
    caret.is_some_and(|c| n.range.start <= c && c <= n.range.end)
}

/// The source ranges of the spans whose markers the caret reveals.
pub fn revealed(nodes: &[Node], caret: Option<usize>) -> Vec<Range<usize>> {
    fn walk(nodes: &[Node], caret: Option<usize>, out: &mut Vec<Range<usize>>) {
        for n in nodes {
            if touches(n, caret) {
                if has_markers(n) {
                    out.push(n.range.clone());
                }
                walk(&n.children, caret, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(nodes, caret, &mut out);
    out
}

/// A link reference definition as Typora's `md-def-link`: the label bold
/// in faint brackets, the destination underlined, the title's quotes faint.
fn link_def_looks(text: &str) -> Vec<(Range<usize>, Look)> {
    let Some([label, dest, title]) = inline::link_ref_def_parts(text) else { return Vec::new() };
    let faint = Look { fg: Paint::Marker, ..Look::default() };
    let mut out = vec![
        (0..label.start, faint),
        (label.clone(), Look { flags: BOLD, ..Look::default() }),
        (label.end..label.end + 2, faint),
        (dest, Look { flags: UNDERLINE, ..Look::default() }),
    ];
    if title.len() >= 2 {
        out.push((title.start..title.start + 1, faint));
        out.push((title.end - 1..title.end, faint));
    }
    out
}

/// The `<br>` tags but the one the caret is at.
fn hidden_brs(nodes: &[Node], caret: Option<usize>) -> Vec<Range<usize>> {
    inline::brs(nodes).into_iter().filter(|n| !touches(n, caret)).map(|n| n.range.clone()).collect()
}

/// `runs` with `range` given `look` instead.
fn overlay(runs: Vec<(Range<usize>, Look)>, range: Range<usize>, look: Look) -> Vec<(Range<usize>, Look)> {
    let mut out = Vec::with_capacity(runs.len() + 1);
    for (r, l) in runs {
        if r.start < range.start {
            out.push((r.start..r.end.min(range.start), l));
        }
        if r.end > range.end {
            out.push((r.start.max(range.end)..r.end, l));
        }
    }
    out.push((range, look));
    out.sort_by_key(|(r, _)| r.start);
    out
}

/// The room the object at a node makes, when it is drawn there.
pub type Room<'a> = &'a dyn Fn(&Node) -> Option<Strut>;

/// Runs of the source (byte ranges, in order, not overlapping) and how each
/// looks. Plain text gets no run. `room` sizes the objects drawn in place
/// of their source; the caret in one shows its source instead.
pub fn looks(nodes: &[Node], len: usize, caret: Option<usize>, room: Room) -> Vec<(Range<usize>, Look)> {
    let mut out = Vec::new();
    walk(nodes, 0..len, Look::default(), caret, room, &mut out);
    out.retain(|(r, l)| !r.is_empty() && *l != Look::default());
    out
}

fn walk(
    nodes: &[Node],
    within: Range<usize>,
    look: Look,
    caret: Option<usize>,
    room: Room,
    out: &mut Vec<(Range<usize>, Look)>,
) {
    let mut at = within.start;
    for n in nodes {
        if n.range.start > at {
            out.push((at..n.range.start, look));
        }
        node(n, look, caret, room, out);
        at = n.range.end;
    }
    if at < within.end {
        out.push((at..within.end, look));
    }
}

fn node(n: &Node, look: Look, caret: Option<usize>, room: Room, out: &mut Vec<(Range<usize>, Look)>) {
    match room(n) {
        // An object drawn over the text: its first character (`!` or `$`)
        // makes the room, the rest takes none.
        Some(strut) if !touches(n, caret) => {
            out.push((n.range.start..n.range.start + 1, Look { strut: Some(strut), ..Look::default() }));
            out.push((n.range.start + 1..n.range.end, HIDE));
        }
        // An image being edited stays after its source, as in Typora: the
        // source's last character (`)` or `]`) makes the room.
        Some(strut) if n.kind.is_image() => {
            let from = out.len();
            source(n, look, caret, room, out);
            let last = n.range.end - 1;
            out.truncate(from + out[from..].iter().take_while(|(r, _)| r.start < last).count());
            if let Some((r, _)) = out[from..].last_mut() {
                r.end = r.end.min(last);
            }
            out.push((last..n.range.end, Look { strut: Some(Strut { tail: true, ..strut }), ..Look::default() }));
        }
        _ => source(n, look, caret, room, out),
    }
}

/// A node as source: its markers hidden or revealed, its content styled.
fn source(n: &Node, look: Look, caret: Option<usize>, room: Room, out: &mut Vec<(Range<usize>, Look)>) {
    // Raw HTML shows as faint monospace source, as Typora shows tags it
    // can't render; comments in their own colour, as Typora's do.
    if matches!(n.kind, Kind::HtmlTag { .. } | Kind::Html | Kind::ImgTag(_)) {
        out.push((n.range.clone(), Look { flags: look.flags | MONO, fg: Paint::Marker, ..look }));
        return;
    }
    if n.kind == Kind::Comment {
        out.push((n.range.clone(), Look { flags: look.flags | MONO, fg: Paint::Comment, ..look }));
        return;
    }
    let inner = content_look(&n.kind, look);
    let markers = if always_source(&n.kind) || touches(n, caret) {
        // An HTML tag's own tags show in monospace, as Typora's do.
        let mono = if matches!(n.kind, Kind::Tag { .. }) { MONO } else { 0 };
        // A footnote's markers stay in its box.
        let bg = if matches!(n.kind, Kind::FootnoteRef { .. }) { inner.bg } else { Paint::None };
        // Only a link's text is the underlined `<a>`.
        let plain = if matches!(n.kind, Kind::Link { .. } | Kind::Autolink { .. }) { UNDERLINE } else { 0 };
        Look { flags: (inner.flags | mono) & !plain, fg: Paint::Marker, bg, ..inner }
    } else {
        HIDE
    };
    if n.content.start > n.range.start {
        out.push((n.range.start..n.content.start, markers));
    }
    if n.children.is_empty() {
        out.push((n.content.clone(), inner));
    } else {
        walk(&n.children, n.content.clone(), inner, caret, room, out);
    }
    if n.range.end > n.content.end {
        out.push((n.content.end..n.range.end, markers));
    }
}

/// An inline image, formula, emoji or footnote reference, drawn over the
/// text once its size is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    pub range: Range<usize>,
    pub kind: &'static str,
    /// The image's destination, the formula's TeX or the emoji.
    pub src: String,
    /// An `<img>` tag's size: its width, height and zoom, CSS values as
    /// written; empty for other objects.
    pub size: [String; 3],
}

impl Object {
    fn new(range: Range<usize>, kind: &'static str, src: &str) -> Object {
        Object { range, kind, src: src.to_string(), size: Default::default() }
    }

    /// What QML reports the object's size under: the same picture at
    /// another size is another object.
    fn key(&self) -> String {
        format!("{}\n{}\n{}", self.kind, self.src, self.size.join("\n"))
    }
}

/// The objects in a block's text, in order. A paragraph that is one image
/// shows it on its own (see `Cache::pictured`), not as an object.
pub fn objects(text: &str, nodes: &[Node]) -> Vec<Object> {
    fn walk(text: &str, nodes: &[Node], lone: Option<&Range<usize>>, out: &mut Vec<Object>) {
        for n in nodes {
            match &n.kind {
                Kind::Image { dest, .. } if lone != Some(&n.range) => out.push(Object::new(n.range.clone(), "image", dest)),
                Kind::ImgTag(tag) if lone != Some(&n.range) => out.push(Object {
                    size: [tag.width.clone(), tag.height.clone(), tag.zoom.clone()],
                    ..Object::new(n.range.clone(), "image", &tag.src)
                }),
                Kind::Math => out.push(Object::new(n.range.clone(), "math", &text[n.content.clone()])),
                Kind::Emoji(e) => out.push(Object::new(n.range.clone(), "emoji", e)),
                // A footnote reference's label, raised in its box.
                Kind::FootnoteRef { label } => out.push(Object::new(n.range.clone(), "footnote", label)),
                // Smart punctuation is drawn as an emoji is, as text.
                Kind::Smart(c) => out.push(Object::new(n.range.clone(), "emoji", &c.to_string())),
                _ => walk(text, &n.children, lone, out),
            }
        }
    }
    let lone = inline::lone_image(text, nodes).map(|n| n.range.clone());
    let mut out = Vec::new();
    walk(text, nodes, lone.as_ref(), &mut out);
    out
}

/// Find's matches from a `hits` property: `[[start, end], …]`.
pub fn hit_ranges(hits: &QString) -> Vec<Range<i32>> {
    let parsed: Vec<[i32; 2]> = serde_json::from_str(&hits.to_string()).unwrap_or_default();
    parsed.into_iter().filter(|[s, e]| s < e).map(|[s, e]| s..e).collect()
}

/// `range` less the `holes` (in order, not overlapping); empty when the
/// range is.
fn subtract(range: Range<i32>, holes: &[Range<i32>]) -> Vec<Range<i32>> {
    let mut out = Vec::new();
    let mut at = range.start;
    for h in holes {
        if h.end <= at || h.start >= range.end {
            continue;
        }
        if h.start > at {
            out.push(at..h.start);
        }
        at = at.max(h.end);
    }
    if at < range.end {
        out.push(at..range.end);
    }
    out
}

/// Object sizes QML reported, by `Object::key`: width, above and below.
type Sizes = HashMap<String, (f64, f64, f64)>;

/// The parsed block and its runs, in UTF-16 positions.
#[derive(Default)]
struct Cache {
    text: String,
    /// The text as the box holds it (`inline::boxed`).
    raw: String,
    /// Where a `<br>`'s `>` is a line break in the box, in `text`.
    cuts: HashSet<usize>,
    /// The `<br>` tags hidden, and where the `>` of those shown is.
    hidden: Vec<Range<usize>>,
    breaks: Vec<i32>,
    nodes: Vec<Node>,
    reveal: Vec<Range<usize>>,
    /// The block is an image showing in place of its source.
    pictured: bool,
    /// The alert marker this block opens with: its kind and length.
    alert: Option<(u8, usize)>,
    /// The caret is on the alert marker, which then shows.
    alert_open: bool,
    /// A link reference definition, not inline markdown.
    link_def: bool,
    /// The document, as the highlighter's `doc`.
    doc: i32,
    objects: Vec<Object>,
    /// Each code span's text in UTF-16, and the characters after which its
    /// box's padding and border go: the opening marker's last and the
    /// text's last.
    boxes: Vec<Range<i32>>,
    pads: Vec<Range<i32>>,
    /// Where each object drawn in the text now has its carrier character.
    placed: Vec<Option<usize>>,
    runs: Vec<(i32, i32, Look)>,
    valid: bool,
}

impl Cache {
    fn parse(&mut self, raw: String, text: String, caret: Option<usize>, image_shown: bool, alert_title: bool, sizes: &Sizes) {
        self.cuts = text.char_indices().zip(raw.chars()).filter(|((_, a), b)| *a == '>' && *b == '\n').map(|((i, _), _)| i).collect();
        self.raw = raw;
        self.alert = alert_title
            .then(|| query::alert_of(&text))
            .flatten()
            .map(|(k, len)| (query::ALERTS.iter().position(|a| *a == k).unwrap_or(0) as u8, len));
        self.nodes = if self.link_def { Vec::new() } else { shared::with_inline(self.doc, |refs, opts| inline::parse(&text, refs, opts)) };
        self.objects = objects(&text, &self.nodes);
        self.text = text;
        self.valid = true;
        self.layout(caret, image_shown, sizes);
    }

    fn objects_json(&self) -> String {
        let items: Vec<String> = self
            .objects
            .iter()
            .map(|o| {
                let [w, h, z] = &o.size;
                format!(
                    "{{\"kind\":\"{}\",\"src\":{},\"key\":{},\"width\":{},\"height\":{},\"zoom\":{}}}",
                    o.kind,
                    json_str(&o.src),
                    json_str(&o.key()),
                    json_str(w),
                    json_str(h),
                    json_str(z)
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    }

    /// Whether the image the block consists of hides its source: it has
    /// loaded and the caret is elsewhere, as in Typora.
    fn pictured(&self, caret: Option<usize>, image_shown: bool) -> bool {
        image_shown && caret.is_none() && inline::lone_image(&self.text, &self.nodes).is_some()
    }

    /// Whether the caret shows the alert marker: on it or just after it.
    fn alert_open(&self, caret: Option<usize>) -> bool {
        self.alert.is_some_and(|(_, len)| caret.is_some_and(|c| c <= len))
    }

    /// The `<br>` tags Preferences ▸ Markdown ▸ Visible <br/> hides: all but
    /// the one the caret is at, which shows as Typora shows it expanded.
    fn hidden_brs(&self, caret: Option<usize>) -> Vec<Range<usize>> {
        if !shared::hide_br(self.doc) || self.pictured || self.link_def {
            return Vec::new();
        }
        hidden_brs(&self.nodes, caret)
    }

    fn layout(&mut self, caret: Option<usize>, image_shown: bool, sizes: &Sizes) {
        self.reveal = revealed(&self.nodes, caret);
        self.alert_open = self.alert_open(caret);
        self.pictured = self.pictured(caret, image_shown);
        let sized: HashMap<usize, Strut> = self
            .objects
            .iter()
            .filter_map(|o| sizes.get(&o.key()).map(|&(w, above, below)| (o.range.start, Strut::new(w, above, below))))
            .collect();
        let room = |n: &Node| match n.kind {
            Kind::Image { .. } | Kind::ImgTag(_) | Kind::Math | Kind::Emoji(_) | Kind::Smart(_) | Kind::FootnoteRef { .. } => {
                sized.get(&n.range.start).copied()
            }
            _ => None,
        };
        let text = &self.text;
        // Byte → UTF-16 for every char boundary, in one pass.
        let mut u16_at = vec![0i32; text.len() + 1];
        let mut u = 0;
        for (i, c) in text.char_indices() {
            u16_at[i] = u;
            u += c.len_utf16() as i32;
        }
        u16_at[text.len()] = u;
        let mut runs = if self.pictured { vec![(0..text.len(), HIDE)] } else { looks(&self.nodes, text.len(), caret, &room) };
        if self.link_def {
            runs = link_def_looks(text);
        }
        // A `<br>`'s `>` is a line break in the box, so the line breaks
        // there as in Typora. Where the box holds the tag as written (no
        // `source`), a hidden one's `>` stays, unseen, as a gap that keeps
        // the words on each side apart.
        self.hidden = self.hidden_brs(caret);
        for r in &self.hidden {
            runs = overlay(runs, r.start..r.end - 1, HIDE);
            if !self.cuts.contains(&(r.end - 1)) {
                runs = overlay(runs, r.end - 1..r.end, GAP);
            }
        }
        self.breaks = if self.link_def || self.pictured {
            Vec::new()
        } else {
            inline::brs(&self.nodes)
                .into_iter()
                .map(|n| n.range.end - 1)
                .filter(|at| self.cuts.contains(at) && !self.hidden.iter().any(|r| r.end - 1 == *at))
                .map(|at| u16_at[at])
                .collect()
        };
        if let Some((kind, len)) = self.alert {
            let look = if self.alert_open {
                Look { flags: BOLD | SMALL, fg: Paint::Alert(kind), ..Look::default() }
            } else {
                HIDE
            };
            runs = overlay(runs, 0..len, look);
        }
        let carriers: HashSet<usize> = runs.iter().filter(|(_, l)| l.strut.is_some()).map(|(r, _)| r.start).collect();
        self.placed = self
            .objects
            .iter()
            .map(|o| [o.range.start, o.range.end - 1].into_iter().find(|at| carriers.contains(at)))
            .collect();
        self.runs = runs.into_iter().map(|(r, l)| (u16_at[r.start], u16_at[r.end], l)).collect();
        let mut spans = Vec::new();
        if !self.pictured && !self.link_def {
            code_spans(&self.nodes, &mut spans);
        }
        self.boxes = spans.iter().map(|r| u16_at[r.start]..u16_at[r.end]).collect();
        self.pads = spans
            .iter()
            .flat_map(|r| {
                let last = text[..r.end].char_indices().next_back().map_or(r.start, |(i, _)| i);
                let marker = text[..r.start].char_indices().next_back().map_or(0, |(i, _)| i);
                [u16_at[marker]..u16_at[r.start], u16_at[last]..u16_at[r.end]]
            })
            .collect();
    }

    fn boxes_json(&self) -> String {
        let items: Vec<String> = self.boxes.iter().map(|r| format!("[{},{}]", r.start, r.end)).collect();
        format!("[{}]", items.join(","))
    }

    fn breaks_json(&self) -> String {
        let items: Vec<String> = self.breaks.iter().map(i32::to_string).collect();
        format!("[{}]", items.join(","))
    }
}

/// The text of every code span in `nodes` that has some.
fn code_spans(nodes: &[Node], out: &mut Vec<Range<usize>>) {
    for n in nodes {
        match n.kind {
            Kind::Code if !n.content.is_empty() && n.content.start > n.range.start => out.push(n.content.clone()),
            _ => code_spans(&n.children, out),
        }
    }
}

pub struct InlineHighlighterRust {
    line_height: f64,
    font_size: f64,
    image_shown: bool,
    objects: QString,
    placing: bool,
    boxes: QString,
    first_indent: f64,
    alert_title: bool,
    link_def: bool,
    hits: QString,
    doc: i32,
    source: QString,
    breaks: QString,
    sizes: Sizes,
    /// Caret in UTF-16, -1 when the block doesn't have it.
    caret: i32,
    cache: Cache,
    dim: Dim,
}

impl Default for InlineHighlighterRust {
    fn default() -> Self {
        InlineHighlighterRust {
            line_height: 0.0,
            font_size: 0.0,
            image_shown: false,
            objects: QString::from("[]"),
            placing: false,
            boxes: QString::from("[]"),
            first_indent: 0.0,
            alert_title: false,
            link_def: false,
            hits: QString::from("[]"),
            doc: -1,
            source: QString::default(),
            breaks: QString::from("[]"),
            sizes: Sizes::new(),
            caret: -1,
            cache: Cache::default(),
            dim: Dim::default(),
        }
    }
}

impl qobject::InlineHighlighter {
    unsafe fn attach(self: Pin<&mut Self>, doc: *mut qobject::QQuickTextDocument) {
        unsafe { self.attach_quick_document(doc) };
    }

    fn refresh(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().cache.valid = false;
        self.rehighlight_all();
    }

    /// What the box shows read as markdown: its text, with `source`'s
    /// characters where the box holds them otherwise (`inline::boxed`).
    fn markdown(&self, raw: &str) -> String {
        inline::shown_source(raw, &self.source.to_string())
    }

    fn reread(self: Pin<&mut Self>) {
        if self.cache.valid && self.markdown(&self.cache.raw) != self.cache.text {
            self.refresh();
        }
    }

    fn dim(mut self: Pin<&mut Self>, blur: bool, keep: i32) {
        let next = Dim::new(blur, keep);
        let old = self.dim;
        if old == next {
            return;
        }
        self.as_mut().rust_mut().dim = next;
        match old.changed(next) {
            Some(lines) => lines.into_iter().for_each(|l| self.as_mut().rehighlight_line(l)),
            None => self.rehighlight_all(),
        }
    }

    fn fix_line_height(self: Pin<&mut Self>) {
        let pixels = *self.line_height();
        let minimum = self.placing;
        let indent = *self.first_indent();
        self.apply_line_height(pixels, minimum, indent);
    }

    /// Lay the cached parse out again and repaint.
    fn relayout(mut self: Pin<&mut Self>) {
        {
            let mut this = self.as_mut().rust_mut();
            let r = &mut *this;
            let caret = (r.caret >= 0).then(|| byte_of(&r.cache.text, r.caret));
            r.cache.layout(caret, r.image_shown, &r.sizes);
        }
        self.rehighlight_all();
    }

    fn object_anchor(&self, index: i32) -> QPointF {
        match self.cache.placed.get(index as usize) {
            Some(&Some(at)) => self.anchor_at(utf16_of(&self.cache.text, at)),
            _ => QPointF::new(f64::NAN, f64::NAN),
        }
    }

    fn object_tail(&self, index: i32) -> QString {
        let i = index as usize;
        match (self.cache.objects.get(i), self.cache.placed.get(i)) {
            (Some(o), Some(&Some(at))) if at != o.range.start => QString::from(&self.cache.text[at..o.range.end]),
            _ => QString::default(),
        }
    }

    fn source_anchor(&self, index: i32) -> QPointF {
        match self.cache.objects.get(index as usize) {
            Some(o) => self.anchor_at(utf16_of(&self.cache.text, o.range.start)),
            None => QPointF::new(f64::NAN, f64::NAN),
        }
    }

    fn baseline_at(&self, pos: i32) -> f64 {
        self.anchor_at(pos).y()
    }

    fn set_object_size(mut self: Pin<&mut Self>, key: &QString, width: f64, above: f64, below: f64) {
        let key = key.to_string();
        let size = (width, above, below);
        if self.sizes.get(&key) == Some(&size) {
            return;
        }
        self.as_mut().rust_mut().sizes.insert(key.clone(), size);
        if self.cache.valid && self.cache.objects.iter().any(|o| o.key() == key) {
            self.relayout();
        }
    }

    fn caret_moved(mut self: Pin<&mut Self>, pos: i32) {
        if self.caret == pos {
            return;
        }
        self.as_mut().rust_mut().caret = pos;
        if !self.cache.valid {
            return;
        }
        let caret = (pos >= 0).then(|| byte_of(&self.cache.text, pos));
        let shown = self.image_shown;
        if revealed(&self.cache.nodes, caret) != self.cache.reveal
            || self.cache.hidden_brs(caret) != self.cache.hidden
            || self.cache.pictured(caret, shown) != self.cache.pictured
            || self.cache.alert_open(caret) != self.cache.alert_open
        {
            self.relayout();
        }
    }

    fn highlight_text(mut self: Pin<&mut Self>, line: &QString) {
        // Spans can cross the lines of a paragraph, so parse the whole
        // block once and format each line from the cached runs.
        let raw = self.document_text().to_string();
        if !self.cache.valid || self.cache.raw != raw {
            let full = self.markdown(&raw);
            let mut this = self.as_mut().rust_mut();
            let r = &mut *this;
            let caret = (r.caret >= 0).then(|| byte_of(&full, r.caret));
            r.cache.link_def = r.link_def;
            r.cache.doc = r.doc;
            r.cache.parse(raw, full, caret, r.image_shown, r.alert_title, &r.sizes);
        }
        let Some(ink) = shared::ink() else { return };
        let paint = |p: Paint| match p {
            Paint::None => 0,
            Paint::Link => ink.link,
            Paint::Code => ink.code,
            Paint::CodeBackground => ink.code_background,
            Paint::Math => ink.math,
            Paint::Mark => ink.mark,
            Paint::Marker => ink.marker,
            Paint::Comment => ink.comment,
            Paint::Footnote => ink.footnote,
            Paint::FootnoteBackground => ink.footnote_background,
            Paint::Alert(k) => ink.alerts[usize::from(k).min(4)],
            Paint::Rgb(c) => c,
        };
        let base = self.block_position();
        let end = base + utf16_of(&line.to_string(), usize::MAX);
        let code_px = if *self.font_size() > 0.0 { *self.font_size() * 0.9 } else { 0.0 };
        let runs: Vec<(i32, i32, Look)> = self
            .cache
            .runs
            .iter()
            .filter(|(s, e, _)| *e > base && *s < end)
            .map(|&(s, e, l)| (s.max(base) - base, e.min(end) - base, l))
            .collect();
        for (s, e, l) in runs {
            if let Some(strut) = l.strut {
                let width = f64::from(strut.width) / 64.0;
                self.as_mut().apply_strut(s, i32::from(strut.step), f64::from(strut.px), width, strut.tail);
                continue;
            }
            let px = if l.flags & MONO != 0 {
                code_px
            } else if l.flags & SMALL != 0 {
                *self.font_size() * 0.9
            } else {
                0.0
            };
            self.as_mut().apply_format(s, e - s, l.flags, px, paint(l.fg), paint(l.bg));
        }
        // Room for a code box's 2px of padding and its border, after the
        // opening marker and the last character, as wide at any zoom.
        let pad = 3.0 * *self.font_size() / 16.0;
        let pads: Vec<Range<i32>> = self.cache.pads.iter().filter(|r| r.end > base && r.start < end).cloned().collect();
        for r in pads {
            let (s, e) = (r.start.max(base) - base, r.end.min(end) - base);
            self.as_mut().add_spacing(s, e - s, pad);
        }
        // Find's matches, but not on markers that take no room or on the
        // characters that make room for objects.
        let holes: Vec<Range<i32>> =
            self.cache.runs.iter().filter(|(_, _, l)| l.flags & HIDDEN != 0 || l.strut.is_some()).map(|&(s, e, _)| s..e).collect();
        for hit in hit_ranges(&self.hits) {
            for r in subtract(hit.start.max(base)..hit.end.min(end), &holes) {
                self.as_mut().add_background(r.start - base, r.end - r.start, ink.find);
            }
        }
        if self.dim.greys(self.block_number()) {
            self.as_mut().dim_line(ink.blur);
        }
        // Last, as QML may call back in.
        let objects = QString::from(&self.cache.objects_json());
        if objects != self.objects {
            self.as_mut().set_objects(objects);
        }
        let boxes = QString::from(&self.cache.boxes_json());
        if boxes != self.boxes {
            self.as_mut().set_boxes(boxes);
        }
        let breaks = QString::from(&self.cache.breaks_json());
        if breaks != self.breaks {
            self.as_mut().set_breaks(breaks);
        }
        let placing = self.cache.placed.iter().any(Option::is_some);
        if placing != self.placing {
            self.as_mut().set_placing(placing);
        }
        self.formatted();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::inline::{Options, Refs};

    fn show(src: &str, caret: Option<usize>) -> String {
        let nodes = inline::parse(src, &Refs::new(), &Options::all());
        looks(&nodes, src.len(), caret, &|_| None)
            .iter()
            .map(|(r, l)| {
                let what =
                    if l.flags & HIDDEN != 0 { "hide".to_string() } else { format!("{:?}/{:?}/{}", l.fg, l.bg, l.flags) };
                format!("{}={}", &src[r.clone()], what)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn find_tints_around_hidden_markers() {
        assert_eq!(subtract(2..10, &[0..3, 5..6, 9..12]), [3..5, 6..9]);
        assert_eq!(subtract(2..4, &[0..5]), Vec::<Range<i32>>::new());
        assert_eq!(subtract(4..4, &[]), Vec::<Range<i32>>::new());
        assert_eq!(hit_ranges(&QString::from("[[1,4],[6,6]]")), [1..4]);
        assert!(hit_ranges(&QString::from("")).is_empty());
    }

    #[test]
    fn hides_markers_away_from_the_caret() {
        assert_eq!(show("a **b** c", None), "**=hide b=None/None/1 **=hide");
        assert_eq!(show("a **b** c", Some(0)), "**=hide b=None/None/1 **=hide");
    }

    #[test]
    fn reveals_markers_at_the_span_edges() {
        assert_eq!(show("a **b** c", Some(2)), "**=Marker/None/1 b=None/None/1 **=Marker/None/1");
        assert_eq!(show("a **b** c", Some(7)), "**=Marker/None/1 b=None/None/1 **=Marker/None/1");
        assert_eq!(show("a **b** c", Some(8)), "**=hide b=None/None/1 **=hide");
    }

    #[test]
    fn nests_styles() {
        assert_eq!(show("[**x**](u)", None), "[=hide **=hide x=Link/None/5 **=hide ](u)=hide");
        // The caret in the bold text reveals the link around it too.
        assert_eq!(
            show("[**x**](u)", Some(3)),
            "[=Marker/None/0 **=Marker/None/5 x=Link/None/5 **=Marker/None/5 ](u)=Marker/None/0"
        );
    }

    #[test]
    fn styles_code_and_html() {
        assert_eq!(show("`x`", None), "`=hide x=Code/None/16 `=hide");
        // Comments stay in sight, faint and monospace, as Typora's do.
        assert_eq!(show("a <!-- c --> b", None), "<!-- c -->=Comment/None/16");
        assert_eq!(show("<u>x</u>", None), "<u>=hide x=None/None/4 </u>=hide");
        // At the caret the tags show in monospace.
        assert_eq!(show("<u>x</u>", Some(1)), "<u>=Marker/None/20 x=None/None/4 </u>=Marker/None/20");
        // Colours a tag's style or a <font> gives.
        assert_eq!(show(r#"<span style="color:red">x</span>"#, None).split(' ').find(|s| s.starts_with("x=")), Some("x=Rgb(4294901760)/None/0"));
        assert_eq!(show(r##"<font color="#0f0">x</font>"##, None).split(' ').find(|s| s.starts_with("x=")), Some("x=Rgb(4278255360)/None/0"));
        assert_eq!(show("<kbd>x</kbd>", None).split(' ').find(|s| s.starts_with("x=")), Some("x=None/CodeBackground/16"));
    }

    #[test]
    fn lists_revealed_spans() {
        let src = "**a** and *b*";
        let nodes = inline::parse(src, &Refs::new(), &Options::all());
        assert_eq!(revealed(&nodes, Some(1)), vec![0..5]);
        assert_eq!(revealed(&nodes, Some(7)), Vec::<Range<usize>>::new());
        assert_eq!(revealed(&nodes, None), Vec::<Range<usize>>::new());
    }

    #[test]
    fn finds_inline_images_and_formulas() {
        let src = "a ![i](x.png) [![b](y.png)](u) $x^2$";
        let nodes = inline::parse(src, &Refs::new(), &Options::all());
        let found: Vec<(&str, String)> = objects(src, &nodes).into_iter().map(|o| (o.kind, o.src)).collect();
        assert_eq!(found, [("image", "x.png".into()), ("image", "y.png".into()), ("math", "x^2".into())]);
        // A paragraph of one image isn't an inline object.
        let nodes = inline::parse("![i](x.png)", &Refs::new(), &Options::all());
        assert_eq!(objects("![i](x.png)", &nodes), []);
    }

    #[test]
    fn a_sized_object_leaves_room_for_itself() {
        let src = "a ![i](x.png) b";
        let nodes = inline::parse(src, &Refs::new(), &Options::all());
        let strut = Strut::new(48.0, 40.0, 0.0);
        let room = |n: &Node| matches!(n.kind, Kind::Image { .. }).then_some(strut);
        let runs = looks(&nodes, src.len(), None, &room);
        assert_eq!(runs[0], (2..3, Look { strut: Some(strut), ..Look::default() }));
        assert_eq!(runs[1], (3..13, HIDE));
        assert_eq!(strut, Strut { step: crate::strut::STEPS, px: 40, width: 48 * 64, tail: false });
        // With the caret in it, the source shows and the image follows it:
        // the `)` makes the room.
        let runs = looks(&nodes, src.len(), Some(5), &room);
        let tail: Vec<_> = runs.iter().filter(|(_, l)| l.strut.is_some()).collect();
        assert_eq!(tail, [&(12..13, Look { strut: Some(Strut { tail: true, ..strut }), ..Look::default() })]);
        assert!(runs.iter().all(|(r, _)| r.end <= 12 || r.start >= 12));
        // A formula being edited is only source.
        let src = "a $x^2$ b";
        let nodes = inline::parse(src, &Refs::new(), &Options::all());
        let room = |n: &Node| matches!(n.kind, Kind::Math).then_some(strut);
        assert!(looks(&nodes, src.len(), Some(4), &room).iter().all(|(_, l)| l.strut.is_none()));
    }

    #[test]
    fn hides_br_tags_away_from_the_caret() {
        let src = "a<br>b<BR/>c";
        let nodes = inline::parse(src, &Refs::new(), &Options::all());
        assert_eq!(hidden_brs(&nodes, None), vec![1..5, 6..11]);
        assert_eq!(hidden_brs(&nodes, Some(3)), vec![6..11]);
    }
}
