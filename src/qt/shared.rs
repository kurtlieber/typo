//! What every block's highlighter reads while painting: the theme's inline
//! colours, and per document (an `Editor`, by its `docId`) its link
//! reference definitions and inline syntax preferences. The Mac's windows
//! share one process, but all of this is only touched on the GUI thread, so
//! thread-local state is enough. The `Theme` and `Editor` objects keep it
//! current and bump a revision that tells the delegates to repaint.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::code::Token;
use crate::md::inline::{Options, Refs};
use crate::palette::{Rgb, Scheme};
use crate::source::Ink as SourceInk;

/// Inline colours as 0xAARRGGBB.
#[derive(Clone, Copy, Debug)]
pub struct Ink {
    pub link: u32,
    pub code: u32,
    pub code_background: u32,
    pub math: u32,
    pub mark: u32,
    /// HTML comments: the theme's yellow, faded, as Typora's are.
    pub comment: u32,
    /// Revealed markdown markers and raw HTML.
    pub marker: u32,
    /// A footnote reference's label and the box behind it.
    pub footnote: u32,
    pub footnote_background: u32,
    /// Behind find's matches: the theme's yellow, faint.
    pub find: u32,
    /// Focus mode's text away from the caret.
    pub blur: u32,
    /// Alert titles, in `query::ALERTS` order.
    pub alerts: [u32; 5],
    /// Code tokens, in `Token::ALL` order.
    pub tokens: [u32; Token::ALL.len()],
    /// Source Code Mode, in `source::Ink::ALL` order.
    pub source: [u32; SourceInk::ALL.len()],
}

impl Ink {
    pub fn of(s: &Scheme) -> Ink {
        Ink {
            link: argb(s.link),
            code: argb(s.code),
            code_background: argb(s.code_background),
            math: argb(s.math),
            mark: argb(s.mark),
            comment: argb(s.palette.get("color3")) & 0x00ff_ffff | 0x9900_0000,
            marker: argb(s.muted),
            footnote: argb(s.foreground),
            footnote_background: argb(s.background.mix(s.foreground, 0.1)),
            find: argb(s.palette.get("yellow")) & 0x00ff_ffff | 0x5900_0000,
            blur: argb(s.blur),
            alerts: crate::palette::ALERT_COLORS.map(|k| argb(s.palette.get(k))),
            tokens: Token::ALL.map(|t| argb(s.palette.get(t.style().0))),
            // Typora's source view: pink headers, green quotes, red tags,
            // its link colour for links and greys for the rest.
            source: SourceInk::ALL.map(|i| {
                argb(match i {
                    SourceInk::Header => s.palette.get("magenta"),
                    SourceInk::Quote => s.palette.get("green"),
                    SourceInk::Rule => s.faint,
                    SourceInk::Code => s.code,
                    SourceInk::Math => s.math,
                    SourceInk::Link => s.link,
                    SourceInk::Url | SourceInk::Atom => s.muted,
                    SourceInk::Tag => s.palette.get("red"),
                })
            }),
        }
    }
}

fn argb(c: Rgb) -> u32 {
    0xff00_0000 | (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32
}

#[derive(Default)]
struct Shared {
    ink: Option<Ink>,
    scheme: Option<Scheme>,
    docs: HashMap<i32, Doc>,
}

#[derive(Default)]
struct Doc {
    refs: Refs,
    opts: Options,
    /// `<br>` tags hide away from the caret (Visible <br/> off).
    hide_br: bool,
}

thread_local! {
    static SHARED: RefCell<Shared> = RefCell::new(Shared::default());
}

pub fn ink() -> Option<Ink> {
    SHARED.with(|s| s.borrow().ink)
}

/// Follow theme `scheme`: its inline colours, and the look of exports.
pub fn set_scheme(scheme: &Scheme) {
    SHARED.with(|s| {
        let mut s = s.borrow_mut();
        s.ink = Some(Ink::of(scheme));
        s.scheme = Some(scheme.clone());
    });
}

/// The theme's colours.
pub fn scheme() -> Scheme {
    SHARED.with(|s| s.borrow().scheme.clone()).unwrap_or_else(Scheme::paper)
}

/// Run `f` with document `doc`'s link references and inline options.
pub fn with_inline<R>(doc: i32, f: impl FnOnce(&Refs, &Options) -> R) -> R {
    SHARED.with(|s| match s.borrow().docs.get(&doc) {
        Some(d) => f(&d.refs, &d.opts),
        None => f(&Refs::new(), &Options::default()),
    })
}

fn edit(doc: i32, f: impl FnOnce(&mut Doc)) {
    SHARED.with(|s| f(s.borrow_mut().docs.entry(doc).or_default()));
}

pub fn set_refs(doc: i32, refs: Refs) {
    edit(doc, |d| d.refs = refs);
}

pub fn set_options(doc: i32, opts: Options) {
    edit(doc, |d| d.opts = opts);
}

pub fn hide_br(doc: i32) -> bool {
    SHARED.with(|s| s.borrow().docs.get(&doc).is_some_and(|d| d.hide_br))
}

pub fn set_hide_br(doc: i32, on: bool) {
    edit(doc, |d| d.hide_br = on);
}

/// Document `doc` is gone (its window closed).
pub fn forget(doc: i32) {
    // At thread exit the state may already be gone, which is as good.
    let _ = SHARED.try_with(|s| s.borrow_mut().docs.remove(&doc));
}
