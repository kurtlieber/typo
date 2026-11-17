//! The find and replace bar over the top right of the note.

use rune::{El, Key, KeyEvent, TextEdit, Tx, col, el, row, text_input};

use super::V;
use super::widgets::{CHEVRON_DOWN, CHEVRON_UP, CLOSE, icon_button, label};
use crate::app::App;
use crate::keys::k;
use crate::theme::{self, mono, sans};

fn field(v: V, key: &'static str, edit: &TextEdit, hint: &str, rep: bool) -> El {
    let p = v.p;
    let input = text_input(edit, sans(13.0, 400.0))
        .key(key)
        .single_line()
        .w_full()
        .color(p.ink)
        .caret(p.ink, p.sel)
        .placeholder(hint, theme::hex("#9a9184"))
        .label(if rep { "Replace with" } else { "Find" })
        .on_edit(move |s: &mut App| if rep { &mut s.find_mut().rep } else { &mut s.find_mut().q })
        .on_input_key(move |s: &mut App, ev: &KeyEvent, tx: &mut Tx| on_field_key(s, ev, tx, rep));
    let input = if rep { input } else { input.on_input_change(|s: &mut App, tx: &mut Tx| s.find_from_caret(tx)) };
    el().key(format!("{key}-box")).grow(1.0).min_w(0.0).h(28.0).px(8.0).center_y().radius(6.0).border(1.0, p.line).child(input)
}

/// Enter goes to the next match (Shift+Enter the one before), or replaces in the replace field.
fn on_field_key(s: &mut App, ev: &KeyEvent, tx: &mut Tx, rep: bool) -> bool {
    match ev.key {
        Key::Escape => {
            s.close_find(tx);
            true
        }
        Key::Enter if rep && ev.mods.cmd => {
            s.replace_all(tx);
            true
        }
        Key::Enter if rep => {
            s.replace_one(tx);
            true
        }
        Key::Enter => {
            s.find_next(ev.mods.shift, tx);
            true
        }
        _ => false,
    }
}

fn text_button(v: V, key: &'static str, t: &'static str, help: String, run: fn(&mut App, &mut Tx)) -> El {
    let p = v.p;
    row()
        .key(key)
        .h(28.0)
        .px(9.0)
        .center()
        .radius(6.0)
        .hover(p.hover)
        .role("button")
        .label(t)
        .help(help)
        .on_press_tx(move |s: &mut App, tx| run(s, tx))
        .child(label(t, sans(12.5, 400.0), p.ink2))
}

/// The bar. In a split it leaves room for the pane's close button.
pub fn bar(v: V, split: bool) -> El {
    let p = v.p;
    let s = v.s;
    let Some(f) = &s.find else { return el() };
    let hits = s.find_matches();
    let count = if f.q.text.is_empty() {
        String::new()
    } else if hits.is_empty() {
        "No results".to_string()
    } else {
        match f.cur {
            Some(i) => format!("{}/{}", i + 1, hits.len()),
            None => format!("{}", hits.len()),
        }
    };
    let case = row()
        .key("find-case")
        .size(28.0, 28.0)
        .center()
        .radius(6.0)
        .fill(if f.case { p.acc.alpha(0.16) } else { rune::Color::TRANSPARENT })
        .hover(p.hover)
        .role("switch")
        .checked(f.case)
        .label("Match case")
        .help("Match case")
        .on_press_tx(|s: &mut App, tx| s.toggle_find_case(tx))
        .child(label("Aa", mono(12.0, 500.0), if f.case { p.acc } else { p.mute }));
    let find_row = row()
        .key("find-row")
        .w_full()
        .gap(4.0)
        .center_y()
        .child(field(v, "find-q", &f.q, "Find", false))
        .child(label(count, mono(11.0, 400.0), p.mute).key("find-count").min_w(0.0))
        .child(case)
        .child(icon_button(p, "find-prev", &CHEVRON_UP, 10.0, &format!("Previous match ({})", k(&["Shift", "Enter"]))).on_press_tx(|s: &mut App, tx| s.find_next(true, tx)))
        .child(icon_button(p, "find-next", &CHEVRON_DOWN, 10.0, "Next match (Enter)").on_press_tx(|s: &mut App, tx| s.find_next(false, tx)))
        .child(icon_button(p, "find-close", &CLOSE, 8.0, "Close (Esc)").on_press_tx(|s: &mut App, tx| s.close_find(tx)));
    let [a, b] = theme::menu_shadows();
    let mut c = col()
        .key("find-bar")
        .pin(None, Some(10.0), Some(if split { 40.0 } else { 16.0 }), None)
        .w(380.0)
        .max_w(v.vw - 32.0)
        .padding(6.0)
        .gap(6.0)
        .radius(9.0)
        .fill(p.bg)
        .box_shadow(a)
        .box_shadow(b)
        .paint_order(5)
        .role("search")
        // A press on the bar stays on it: the note under it would move the caret.
        .on_press(|_: &mut App| {})
        .child(find_row);
    if f.replace {
        c = c.child(
            row()
                .key("replace-row")
                .w_full()
                .gap(4.0)
                .center_y()
                .child(field(v, "find-rep", &f.rep, "Replace", true))
                .child(text_button(v, "find-replace", "Replace", "Replace this match (Enter)".into(), |s, tx| s.replace_one(tx)))
                .child(text_button(v, "find-all", "All", format!("Replace all ({})", k(&["Mod", "Enter"])), |s, tx| {
                    s.replace_all(tx);
                })),
        );
    }
    c
}
