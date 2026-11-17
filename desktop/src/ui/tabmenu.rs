//! The menu a right-click on a tab opens.

use rune::{El, Tx, col, el, icon, row};

use super::V;
use super::widgets::{CHEVRON_RIGHT, label};
use crate::app::App;
use crate::keys;
use crate::tabs::Dir;
use crate::theme::{self, sans};

type Run = Box<dyn Fn(&mut App, &mut Tx)>;

enum Item {
    Row { key: &'static str, label: &'static str, hint: String, danger: bool, run: Option<Run> },
    Copy,
    Line,
}

const W: f32 = 230.0;
const ROW: f32 = 30.0;
const LINE: f32 = 9.0;
const SUB_W: f32 = 170.0;

fn row_item(key: &'static str, label: &'static str, run: Option<Run>) -> Item {
    Item::Row { key, label, hint: String::new(), danger: false, run }
}

pub fn tab_menu(v: V) -> El {
    let p = v.p;
    let s = v.s;
    let Some(m) = &s.tab_menu else { return el() };
    let path = m.path.clone();
    let i = s.tabs.iter().position(|t| *t == path).unwrap_or(0);
    let last = i + 1 >= s.tabs.len();
    // A locked note stays in the main window, and find needs its text.
    let locked = s.note_locked(&path) || s.note_unlocked(&path);
    let shut = s.note_locked(&path);
    let on = |f: fn(&mut App, &str, &mut Tx)| -> Option<Run> {
        let p = path.clone();
        Some(Box::new(move |s: &mut App, tx: &mut Tx| f(s, &p, tx)))
    };
    let items = vec![
        row_item("tmn-close", "Close", on(|s, p, _| s.close_tab(p))),
        row_item("tmn-others", "Close others", if s.tabs.len() > 1 { on(|s, p, _| s.close_others(p)) } else { None }),
        row_item("tmn-after", "Close tabs after", if last { None } else { on(|s, p, _| s.close_tabs_after(p)) }),
        row_item("tmn-all", "Close all", on(|s, _, _| s.close_all())),
        Item::Line,
        row_item("tmn-move", "Move to new window", if locked { None } else { on(|s, p, _| s.open_note_window(p, true)) }),
        row_item("tmn-split-right", "Split right", on(|s, p, tx| s.split_tab(p, Dir::Right, tx))),
        row_item("tmn-split-down", "Split down", on(|s, p, tx| s.split_tab(p, Dir::Down, tx))),
        row_item("tmn-window", "Open in new window", if locked { None } else { on(|s, p, _| s.open_note_window(p, false)) }),
        Item::Line,
        Item::Row { key: "tmn-find", label: "Find…", hint: keys::k(&["Mod", "F"]), danger: false, run: if shut { None } else { on(|s, p, tx| s.find_in(p, false, tx)) } },
        Item::Row { key: "tmn-replace", label: "Replace…", hint: keys::replace_keys(), danger: false, run: if shut { None } else { on(|s, p, tx| s.find_in(p, true, tx)) } },
        Item::Copy,
        Item::Line,
        Item::Row { key: "tmn-delete", label: "Delete file", hint: String::new(), danger: true, run: on(|s, p, _| s.delete_tab_note(p)) },
    ];
    let h = 10.0 + items.iter().map(|it| if matches!(it, Item::Line) { LINE } else { ROW }).sum::<f32>();
    let x = m.x.min(v.vw - W - 8.0).max(8.0);
    let y = m.y.min(v.vh - h - 8.0).max(8.0);
    let [a, b] = theme::menu_shadows();
    let mut panel = col().key("tab-menu").absolute(x, y).w(W).padding(5.0).radius(9.0).fill(p.bg).box_shadow(a).box_shadow(b).role("menu").modal(true, |s: &mut App| s.tab_menu = None);
    let mut top = 5.0;
    let mut copy_top = 0.0;
    for it in items {
        match it {
            Item::Line => {
                panel = panel.child(el().w_full().h(LINE).py(4.0).child(el().w_full().h(1.0).fill(p.line)));
                top += LINE;
            }
            Item::Copy => {
                copy_top = top;
                let open = m.copy_open;
                panel = panel.child(
                    row()
                        .key("tmn-copy")
                        .w_full()
                        .h(ROW)
                        .px(10.0)
                        .center_y()
                        .radius(6.0)
                        .fill(if open { p.hover } else { rune::Color::TRANSPARENT })
                        .hover(p.hover)
                        .role("menuitem")
                        .label("Copy path")
                        .on_hover_change(|s: &mut App, on, _| {
                            if on && let Some(m) = &mut s.tab_menu {
                                m.copy_open = true;
                            }
                        })
                        .on_press(|s: &mut App| {
                            if let Some(m) = &mut s.tab_menu {
                                m.copy_open = !m.copy_open;
                            }
                        })
                        .child(label("Copy path", sans(13.0, 400.0), p.ink).grow(1.0))
                        .child(icon(&CHEVRON_RIGHT, 8.0).color(p.mute)),
                );
                top += ROW;
            }
            Item::Row { key, label: t, hint, danger, run } => {
                let enabled = run.is_some();
                let ink = if !enabled { p.mute } else if danger { p.err } else { p.ink };
                let mut r = row()
                    .key(key)
                    .w_full()
                    .h(ROW)
                    .px(10.0)
                    .gap(12.0)
                    .center_y()
                    .radius(6.0)
                    .role("menuitem")
                    .label(t)
                    .disabled(!enabled)
                    .on_hover_change(|s: &mut App, on, _| {
                        if on && let Some(m) = &mut s.tab_menu {
                            m.copy_open = false;
                        }
                    })
                    .child(label(t, sans(13.0, 400.0), ink).grow(1.0));
                if !hint.is_empty() {
                    r = r.child(label(hint, sans(12.0, 400.0), p.mute));
                }
                if let Some(run) = run {
                    r = r.hover(p.hover).on_press_tx(move |s: &mut App, tx| {
                        s.tab_menu = None;
                        run(s, tx);
                    });
                }
                panel = panel.child(r);
                top += ROW;
            }
        }
    }
    if m.copy_open {
        // Beside the menu, on the left when there is no room on the right.
        let sx = if x + W + SUB_W - 4.0 > v.vw - 8.0 { -SUB_W + 4.0 } else { W - 4.0 };
        let sub_y = (copy_top - 5.0).min(v.vh - y - (10.0 + 2.0 * ROW) - 8.0);
        let sub = |key: &'static str, t: &'static str, md: bool| {
            let p2 = path.clone();
            row()
                .key(key)
                .w_full()
                .h(ROW)
                .px(10.0)
                .center_y()
                .radius(6.0)
                .hover(p.hover)
                .role("menuitem")
                .label(t)
                .on_press_tx(move |s: &mut App, tx| {
                    s.tab_menu = None;
                    s.copy_path(&p2, md, tx);
                })
                .child(label(t, sans(13.0, 400.0), p.ink))
        };
        panel = panel.child(
            col()
                .key("tmn-copy-menu")
                .absolute(sx, sub_y)
                .w(SUB_W)
                .padding(5.0)
                .radius(9.0)
                .fill(p.bg)
                .box_shadow(a)
                .box_shadow(b)
                .role("menu")
                .child(sub("tmn-copy-abs", "Absolute path", false))
                .child(sub("tmn-copy-md", "Markdown link", true)),
        );
    }
    el().key("tab-menu-layer").pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).paint_order(55).child(panel)
}
